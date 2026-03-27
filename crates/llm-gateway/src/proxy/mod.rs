//! Internal HTTP Proxy Module
//!
//! LLM Gateway 的传输子模块，用于统一上游连接、超时、重试与代理出口。
//!
//! ## 核心职责
//! - 统一 HTTP 连接池管理
//! - 多代理出口支持（按 Provider/账号选择代理）
//! - 统一超时和重试配置
//! - 请求追踪和监控
//!
//! ## 架构定位
//! 根据 ARCHTECURE.md：
//! > `Internal HTTP Proxy Module` 不是独立业务层，而是 `LLM Gateway` 的传输子模块，
//! > 用于统一上游连接、超时、重试与代理出口。

pub mod client;
pub mod config;
pub mod request;
mod selector;

pub use client::HttpClient;
pub use config::ProxyConfig;
pub use request::ProxyRequest;
pub use selector::ProxySelector;

use std::collections::HashMap;
use std::sync::Arc;

/// Internal HTTP Proxy
///
/// 统一管理上游 HTTP 连接，支持：
/// - 多代理出口
/// - 连接池复用
/// - 统一超时控制
/// - 请求追踪
#[derive(Debug, Clone)]
pub struct HttpProxy {
    /// 默认 HTTP 客户端（无代理）
    default_client: Arc<HttpClient>,
    /// 代理选择器
    proxy_selector: ProxySelector,
    /// 全局配置
    config: ProxyConfig,
}

impl HttpProxy {
    /// 创建新的 HTTP Proxy
    pub fn new(config: ProxyConfig) -> Self {
        let default_client = Arc::new(HttpClient::new(&config, None));
        let proxy_selector = ProxySelector::new();

        Self {
            default_client,
            proxy_selector,
            config,
        }
    }

    /// 创建带代理配置的 HTTP Proxy
    pub fn with_proxies(config: ProxyConfig, proxies: HashMap<String, String>) -> Self {
        let default_client = Arc::new(HttpClient::new(&config, None));
        let proxy_selector = ProxySelector::with_proxies(proxies);

        Self {
            default_client,
            proxy_selector,
            config,
        }
    }

    /// 获取默认客户端（无代理）
    pub fn default_client(&self) -> &Arc<HttpClient> {
        &self.default_client
    }

    /// 根据 Provider 名称获取客户端
    ///
    /// 如果该 Provider 配置了专用代理，返回使用该代理的客户端；
    /// 否则返回默认客户端。
    pub fn client_for_provider(&self, provider: &str) -> Arc<HttpClient> {
        if let Some(proxy_url) = self.proxy_selector.select(provider) {
            // 为每个代理 URL 创建或获取客户端
            Arc::new(HttpClient::new(&self.config, Some(proxy_url)))
        } else {
            Arc::clone(&self.default_client)
        }
    }

    /// 根据 Provider 和账号获取客户端
    ///
    /// 支持更细粒度的代理选择策略：
    /// 1. 账号级代理（最高优先级）
    /// 2. Provider 级代理
    /// 3. 默认客户端
    pub fn client_for_provider_and_account(
        &self,
        provider: &str,
        account_id: Option<uuid::Uuid>,
    ) -> Arc<HttpClient> {
        // 优先检查账号级代理
        if let Some(id) = account_id
            && let Some(proxy_url) = self.proxy_selector.select_for_account(provider, id)
        {
            return Arc::new(HttpClient::new(&self.config, Some(proxy_url)));
        }

        // 回退到 Provider 级代理
        self.client_for_provider(provider)
    }

    /// 添加代理规则
    pub fn add_proxy(&mut self, pattern: String, proxy_url: String) {
        self.proxy_selector.add_proxy(pattern, proxy_url);
    }

    /// 获取配置
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

impl Default for HttpProxy {
    fn default() -> Self {
        Self::new(ProxyConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_proxy_new() {
        let proxy = HttpProxy::new(ProxyConfig::default());
        assert!(proxy.default_client().is_shared());
    }

    #[test]
    fn test_http_proxy_with_proxies() {
        let mut proxies = HashMap::new();
        proxies.insert("openai".to_string(), "http://proxy1:8080".to_string());
        proxies.insert("claude".to_string(), "http://proxy2:8080".to_string());

        let proxy = HttpProxy::with_proxies(ProxyConfig::default(), proxies);

        // 有代理的 Provider
        let client = proxy.client_for_provider("openai");
        assert!(client.is_shared());

        // 无代理的 Provider
        let client = proxy.client_for_provider("unknown");
        assert!(client.is_shared());
    }

    #[test]
    fn test_add_proxy() {
        let mut proxy = HttpProxy::new(ProxyConfig::default());
        proxy.add_proxy("deepseek".to_string(), "http://proxy:8080".to_string());

        let client = proxy.client_for_provider("deepseek");
        assert!(client.is_shared());
    }
}
