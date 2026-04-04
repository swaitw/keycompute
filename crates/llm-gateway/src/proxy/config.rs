//! HTTP Proxy 配置
//!
//! 定义代理模块的全局配置项

use std::time::Duration;

/// HTTP Proxy 配置
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// 连接超时时间
    pub connect_timeout: Duration,
    /// 请求超时时间
    pub request_timeout: Duration,
    /// 流式请求超时时间（通常更长）
    pub stream_timeout: Duration,
    /// 连接池最大空闲连接数
    pub pool_max_idle_per_host: usize,
    /// 连接池空闲超时
    pub pool_idle_timeout: Duration,
    /// 是否启用 TCP keepalive
    pub tcp_keepalive: Option<Duration>,
    /// 是否启用请求追踪
    pub enable_tracing: bool,
    /// 用户代理
    pub user_agent: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(120),
            stream_timeout: Duration::from_secs(600), // 10 分钟，流式请求可能很长
            // 使用小连接池，短超时，避免复用已关闭的连接
            pool_max_idle_per_host: 2,
            pool_idle_timeout: Duration::from_secs(10),
            tcp_keepalive: Some(Duration::from_secs(15)),
            enable_tracing: true,
            user_agent: format!("KeyCompute-Gateway/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl ProxyConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置连接超时
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// 设置请求超时
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// 设置流式请求超时
    pub fn with_stream_timeout(mut self, timeout: Duration) -> Self {
        self.stream_timeout = timeout;
        self
    }

    /// 设置连接池最大空闲连接数
    pub fn with_pool_max_idle(mut self, max: usize) -> Self {
        self.pool_max_idle_per_host = max;
        self
    }

    /// 设置是否启用追踪
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// 设置用户代理
    pub fn with_user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = agent.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.request_timeout, Duration::from_secs(120));
        assert_eq!(config.stream_timeout, Duration::from_secs(600));
        assert!(config.enable_tracing);
    }

    #[test]
    fn test_config_builder() {
        let config = ProxyConfig::new()
            .with_connect_timeout(Duration::from_secs(5))
            .with_request_timeout(Duration::from_secs(60))
            .with_stream_timeout(Duration::from_secs(300))
            .with_tracing(false);

        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(60));
        assert_eq!(config.stream_timeout, Duration::from_secs(300));
        assert!(!config.enable_tracing);
    }
}
