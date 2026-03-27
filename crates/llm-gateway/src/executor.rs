//! Gateway 执行器
//!
//! 核心执行入口，控制 retry/fallback/streaming 生命周期。
//!
//! 健康状态集成：
//! - 执行成功时调用 ProviderHealthStore::record_success
//! - 执行失败时调用 ProviderHealthStore::record_failure
//! - 延迟时间从请求开始计算
//!
//! Internal HTTP Proxy 集成：
//! - 统一上游连接管理
//! - 多代理出口支持
//! - 请求追踪

use crate::{GatewayConfig, HttpProxy, streaming::StreamPipeline};
use futures::StreamExt;
use keycompute_provider_trait::{HttpTransport, ProviderAdapter, StreamEvent, UpstreamRequest};
use keycompute_runtime::{AccountStateStore, ProviderHealthStore};
use keycompute_types::{ExecutionPlan, ExecutionTarget, KeyComputeError, RequestContext, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Gateway 执行器
///
/// 唯一执行层，负责：
/// 1. 执行请求到上游 Provider
/// 2. 处理 retry 和 fallback
/// 3. 管理 streaming 生命周期
/// 4. 更新运行时状态（账号状态 + Provider 健康状态）
///
/// Internal HTTP Proxy 集成：
/// - 统一连接池管理
/// - 多代理出口支持
/// - 请求追踪
#[derive(Debug)]
pub struct GatewayExecutor {
    #[allow(dead_code)]
    config: GatewayConfig,
    providers: HashMap<String, Arc<dyn ProviderAdapter>>,
    /// Internal HTTP Proxy（统一上游连接管理）
    http_proxy: Option<Arc<HttpProxy>>,
}

impl GatewayExecutor {
    /// 创建新的执行器
    pub fn new(
        config: GatewayConfig,
        providers: HashMap<String, Arc<dyn ProviderAdapter>>,
    ) -> Self {
        Self {
            config,
            providers,
            http_proxy: None,
        }
    }

    /// 创建带 HTTP Proxy 的执行器
    pub fn with_proxy(
        config: GatewayConfig,
        providers: HashMap<String, Arc<dyn ProviderAdapter>>,
        http_proxy: Arc<HttpProxy>,
    ) -> Self {
        Self {
            config,
            providers,
            http_proxy: Some(http_proxy),
        }
    }

    /// 获取 HTTP Proxy
    pub fn http_proxy(&self) -> Option<&Arc<HttpProxy>> {
        self.http_proxy.as_ref()
    }

    /// 设置 HTTP Proxy
    pub fn set_http_proxy(&mut self, proxy: Arc<HttpProxy>) {
        self.http_proxy = Some(proxy);
    }

    /// 执行请求（唯一执行入口）
    ///
    /// 执行流程：
    /// 1. 尝试 primary target
    /// 2. 失败则 fallback 到下一个 target
    /// 3. 成功后更新账号状态和 Provider 健康状态
    ///
    /// # 参数
    /// - `ctx`: 请求上下文
    /// - `plan`: 执行计划（包含 primary 和 fallback chain）
    /// - `account_states`: 账号状态存储
    /// - `provider_health`: Provider 健康状态存储（可选，用于被动记录健康状态）
    pub async fn execute(
        &self,
        ctx: Arc<RequestContext>,
        plan: ExecutionPlan,
        account_states: Arc<AccountStateStore>,
        provider_health: Option<Arc<ProviderHealthStore>>,
    ) -> Result<mpsc::Receiver<StreamEvent>> {
        let (tx, rx) = mpsc::channel(100);

        // 构建 target 链：primary + fallback
        let mut targets = vec![plan.primary];
        targets.extend(plan.fallback_chain);

        let mut last_error = None;
        let _start_time = Instant::now();

        for target in targets {
            let target_start = Instant::now();
            match self.try_execute(&ctx, &target, tx.clone()).await {
                Ok(()) => {
                    // 成功：标记账号状态
                    account_states.mark_success(target.account_id);

                    // 成功：更新 Provider 健康状态
                    let latency_ms = target_start.elapsed().as_millis() as u64;
                    if let Some(ref health_store) = provider_health {
                        health_store.record_success(&target.provider, latency_ms);
                    }

                    tracing::info!(
                        request_id = %ctx.request_id,
                        provider = %target.provider,
                        latency_ms = latency_ms,
                        "Request executed successfully"
                    );
                    return Ok(rx);
                }
                Err(e) => {
                    // 失败：标记错误，继续 fallback
                    account_states.mark_error(target.account_id);

                    // 失败：更新 Provider 健康状态
                    if let Some(ref health_store) = provider_health {
                        health_store.record_failure(&target.provider);
                    }

                    tracing::warn!(
                        request_id = %ctx.request_id,
                        provider = %target.provider,
                        error = %e,
                        "Request failed, trying fallback"
                    );
                    last_error = Some(e);
                }
            }
        }

        // 所有 target 都失败
        Err(last_error.unwrap_or(KeyComputeError::RoutingFailed))
    }

    /// 尝试执行单个 target
    async fn try_execute(
        &self,
        ctx: &RequestContext,
        target: &ExecutionTarget,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<()> {
        // 获取 Provider
        let provider = self.providers.get(&target.provider).ok_or_else(|| {
            KeyComputeError::Internal(format!("Provider {} not found", target.provider))
        })?;

        // 获取 HTTP 传输层（优先使用 HttpProxy 中的客户端）
        let transport: Arc<dyn HttpTransport> = if let Some(ref proxy) = self.http_proxy {
            Arc::clone(proxy.default_client()) as Arc<dyn HttpTransport>
        } else {
            // 使用默认传输
            Arc::new(keycompute_provider_trait::DefaultHttpTransport::new())
        };

        // 构建上游请求
        let request = self.build_upstream_request(ctx, target);

        // 执行流式请求（传入 transport）
        let mut stream = provider.stream_chat(transport.as_ref(), request).await?;

        // 流处理管道
        let mut pipeline = StreamPipeline::new(ctx.request_id);

        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::Delta {
                    content,
                    finish_reason,
                } => {
                    // 累积 tokens（简化估算）
                    let tokens = Self::estimate_tokens(&content);
                    ctx.usage.add_output(tokens);

                    // 转发给客户端
                    let event = StreamEvent::Delta {
                        content,
                        finish_reason,
                    };
                    pipeline.process_event(&event);
                    tx.send(event)
                        .await
                        .map_err(|_| KeyComputeError::Internal("Send error".into()))?;
                }
                StreamEvent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    // Provider 报告的用量（优先级更高）
                    ctx.usage.set_input(input_tokens);
                    // 覆盖输出的 token 计数
                    let current_output = ctx.usage.snapshot().1;
                    if output_tokens > current_output {
                        ctx.usage.add_output(output_tokens - current_output);
                    }
                }
                StreamEvent::Done => {
                    tx.send(StreamEvent::Done)
                        .await
                        .map_err(|_| KeyComputeError::Internal("Send error".into()))?;
                    break;
                }
                StreamEvent::Error { message } => {
                    return Err(KeyComputeError::ProviderError(message));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 构建上游请求
    fn build_upstream_request(
        &self,
        ctx: &RequestContext,
        target: &ExecutionTarget,
    ) -> UpstreamRequest {
        let messages: Vec<keycompute_provider_trait::UpstreamMessage> = ctx
            .messages
            .iter()
            .map(|m| keycompute_provider_trait::UpstreamMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        UpstreamRequest {
            endpoint: target.endpoint.clone(),
            upstream_api_key: target.upstream_api_key.clone(),
            model: ctx.model.clone(),
            messages,
            stream: ctx.stream,
            max_tokens: None,
            temperature: None,
            top_p: None,
        }
    }

    /// 精确计算 token 数
    ///
    /// 使用 tiktoken-rs 库的 o200k_base tokenizer（支持 GPT-4o, o1, o3 等模型）
    /// 提供与 OpenAI API 完全一致的 token 计数
    fn estimate_tokens(content: &str) -> u32 {
        if content.is_empty() {
            return 0;
        }

        // 使用 o200k_base tokenizer (GPT-4o, o1, o3 等模型)
        // singleton 模式避免重复加载词表
        let bpe = tiktoken_rs::o200k_base_singleton();
        bpe.encode_with_special_tokens(content).len() as u32
    }

    /// 获取所有 Provider 名称列表
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// 检查是否存在指定的 Provider
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// 获取 Provider 数量
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycompute_types::{Message, PricingSnapshot, UsageAccumulator};
    use rust_decimal::Decimal;

    #[allow(dead_code)]
    fn create_test_context() -> RequestContext {
        RequestContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            produce_ai_key_id: uuid::Uuid::new_v4(),
            model: "gpt-4o".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: true,
            pricing_snapshot: PricingSnapshot {
                model_name: "gpt-4o".to_string(),
                currency: "CNY".to_string(),
                input_price_per_1k: Decimal::from(1),
                output_price_per_1k: Decimal::from(2),
            },
            usage: UsageAccumulator::default(),
            started_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_gateway_executor_new() {
        let config = GatewayConfig::default();
        let providers = HashMap::new();
        let executor = GatewayExecutor::new(config, providers);
        assert_eq!(executor.config.max_retries, 3);
    }

    #[test]
    fn test_estimate_tokens_english() {
        // 使用 tiktoken-rs o200k_base 精确计数
        // "Hello" = 1 token
        assert_eq!(GatewayExecutor::estimate_tokens("Hello"), 1);
        // "Hello World" = 2 tokens
        assert_eq!(GatewayExecutor::estimate_tokens("Hello World"), 2);
        // 100 个 'a' 约 25 tokens
        assert!(GatewayExecutor::estimate_tokens("a".repeat(100).as_str()) > 0);
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        // 中文 token 计数（tiktoken 精确计算）
        // 中文字符通常每个 1-2 tokens
        assert!(GatewayExecutor::estimate_tokens("你好") > 0);
        assert!(GatewayExecutor::estimate_tokens("你好世界") > 0);
        assert!(GatewayExecutor::estimate_tokens("你好世界测试") > 0);
    }

    #[test]
    fn test_estimate_tokens_mixed() {
        // 混合：英文 + 中文
        assert!(GatewayExecutor::estimate_tokens("Hello你好") > 0);
        assert!(GatewayExecutor::estimate_tokens("Hello World你好世界") > 0);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(GatewayExecutor::estimate_tokens(""), 0);
    }
}
