//! 路由调试处理器
//!
//! 用于查看路由决策过程和状态

use crate::{
    error::{ApiError, Result},
    extractors::AuthExtractor,
    state::AppState,
};
use axum::{
    Json,
    extract::{Query, State},
};
use keycompute_types::{RequestContext, UsageAccumulator};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 路由调试请求
#[derive(Debug, Deserialize)]
pub struct RoutingDebugQuery {
    /// 模型名称
    pub model: String,
}

/// 路由目标信息
#[derive(Debug, Serialize)]
pub struct RoutingTargetInfo {
    /// Provider 名称
    pub provider: String,
    /// 账号 ID
    pub account_id: Uuid,
    /// 端点
    pub endpoint: String,
}

/// Provider 状态信息
#[derive(Debug, Serialize)]
pub struct ProviderStatusInfo {
    /// Provider 名称
    pub provider: String,
    /// 是否健康
    pub is_healthy: bool,
    /// 账号数量
    pub account_count: usize,
    /// 状态描述
    pub status: String,
}

/// 路由调试响应
#[derive(Debug, Serialize)]
pub struct RoutingDebugResponse {
    /// 请求 ID
    pub request_id: Uuid,
    /// 是否成功路由
    pub routed: bool,
    /// 主目标（路由成功时有值）
    pub primary: Option<RoutingTargetInfo>,
    /// 备用链路（路由成功时有值）
    pub fallback_chain: Vec<RoutingTargetInfo>,
    /// 使用的定价
    pub pricing: PricingInfo,
    /// 所有 Provider 状态（用于诊断）
    pub provider_status: Vec<ProviderStatusInfo>,
    /// 提示信息
    pub message: Option<String>,
}

/// 定价信息
#[derive(Debug, Serialize)]
pub struct PricingInfo {
    /// 模型名称
    pub model_name: String,
    /// 货币
    pub currency: String,
    /// 输入价格（每 1K tokens）
    pub input_price_per_1k: String,
    /// 输出价格（每 1K tokens）
    pub output_price_per_1k: String,
}

/// 路由调试接口
///
/// 模拟一个请求，返回路由决策结果（不实际执行）
/// 即使路由失败也会返回 200，包含详细的诊断信息
pub async fn debug_routing(
    State(state): State<AppState>,
    auth: AuthExtractor,
    Query(query): Query<RoutingDebugQuery>,
) -> Result<Json<RoutingDebugResponse>> {
    use keycompute_db::models::Account;

    // 1. 构建 PricingSnapshot
    let pricing = state
        .pricing
        .create_snapshot(&query.model, &auth.tenant_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create pricing snapshot: {}", e)))?;

    // 2. 构建模拟的 RequestContext
    let request_id = Uuid::new_v4();
    let ctx = RequestContext {
        request_id,
        user_id: auth.user_id,
        tenant_id: auth.tenant_id,
        produce_ai_key_id: auth.produce_ai_key_id,
        model: query.model.clone(),
        messages: vec![],
        stream: true,
        pricing_snapshot: pricing.clone(),
        usage: UsageAccumulator::default(),
        started_at: chrono::Utc::now(),
    };

    // 3. 获取所有配置的 provider 列表
    let all_providers: Vec<String> = state.routing.configured_providers().to_vec();
    let healthy_providers = state.routing.healthy_providers();
    let healthy_set: std::collections::HashSet<String> =
        healthy_providers.iter().cloned().collect();

    // 4. 查询每个 provider 的账号数量
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    // 获取所有启用的账号
    let all_accounts = Account::find_enabled_all(pool).await.unwrap_or_default();

    // 按 provider 统计账号数量
    let mut provider_account_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for account in &all_accounts {
        *provider_account_counts
            .entry(account.provider.clone())
            .or_insert(0) += 1;
    }

    let mut provider_status = Vec::new();
    for provider in all_providers {
        let is_healthy = healthy_set.contains(&provider);
        let account_count = provider_account_counts.get(&provider).copied().unwrap_or(0);

        let status = if account_count == 0 {
            "未配置账号".to_string()
        } else if !is_healthy {
            "Provider 不健康".to_string()
        } else {
            format!("{} 个账号", account_count)
        };

        provider_status.push(ProviderStatusInfo {
            provider: provider.clone(),
            is_healthy,
            account_count,
            status,
        });
    }

    // 5. 执行路由（只读）
    match state.routing.route(&ctx).await {
        Ok(plan) => {
            // 路由成功
            let response = RoutingDebugResponse {
                request_id,
                routed: true,
                primary: Some(RoutingTargetInfo {
                    provider: plan.primary.provider.clone(),
                    account_id: plan.primary.account_id,
                    endpoint: plan.primary.endpoint.clone(),
                }),
                fallback_chain: plan
                    .fallback_chain
                    .iter()
                    .map(|t| RoutingTargetInfo {
                        provider: t.provider.clone(),
                        account_id: t.account_id,
                        endpoint: t.endpoint.clone(),
                    })
                    .collect(),
                pricing: PricingInfo {
                    model_name: pricing.model_name,
                    currency: pricing.currency,
                    input_price_per_1k: pricing.input_price_per_1k.to_string(),
                    output_price_per_1k: pricing.output_price_per_1k.to_string(),
                },
                provider_status,
                message: None,
            };

            tracing::info!(
                request_id = %request_id,
                primary_provider = %plan.primary.provider,
                fallback_count = plan.fallback_chain.len(),
                "Routing debug completed"
            );

            Ok(Json(response))
        }
        Err(keycompute_types::KeyComputeError::RoutingFailed) => {
            // 路由失败，但仍返回诊断信息
            let response = RoutingDebugResponse {
                request_id,
                routed: false,
                primary: None,
                fallback_chain: vec![],
                pricing: PricingInfo {
                    model_name: pricing.model_name,
                    currency: pricing.currency,
                    input_price_per_1k: pricing.input_price_per_1k.to_string(),
                    output_price_per_1k: pricing.output_price_per_1k.to_string(),
                },
                provider_status,
                message: Some(format!(
                    "模型 '{}' 没有可用的路由目标。请检查：1) 是否已配置对应 Provider 的账号；2) Provider 是否健康；3) 模型名称是否正确。",
                    query.model
                )),
            };

            tracing::warn!(
                request_id = %request_id,
                model = %query.model,
                "Routing debug failed: no available targets"
            );

            Ok(Json(response))
        }
        Err(e) => Err(ApiError::Internal(format!("Routing failed: {}", e))),
    }
}

/// Provider 健康状态响应
#[derive(Debug, Serialize)]
pub struct ProviderHealthResponse {
    /// 可用 Provider 列表
    pub healthy_providers: Vec<String>,
    /// 账号状态存储中的账号数量
    pub account_count: usize,
}

/// 获取 Provider 健康状态
pub async fn get_provider_health(
    State(state): State<AppState>,
    _auth: AuthExtractor,
) -> Result<Json<ProviderHealthResponse>> {
    let providers = state.routing.healthy_providers().to_vec();

    Ok(Json(ProviderHealthResponse {
        healthy_providers: providers,
        account_count: 0, // TODO: 从 AccountStateStore 获取实际数量
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_debug_query_deserialize() {
        let json = r#"{"model": "gpt-4o"}"#;
        let query: RoutingDebugQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.model, "gpt-4o");
    }

    #[test]
    fn test_routing_target_info_serialize() {
        let info = RoutingTargetInfo {
            provider: "openai".to_string(),
            account_id: Uuid::new_v4(),
            endpoint: "https://api.openai.com/v1".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("openai"));
    }
}
