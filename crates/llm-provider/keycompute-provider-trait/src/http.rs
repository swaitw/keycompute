//! HTTP 传输层抽象
//!
//! 定义统一的 HTTP 客户端接口，供 Provider Adapter 使用。
//! 具体实现由 llm-gateway 提供，避免循环依赖。

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use keycompute_types::Result;
use std::pin::Pin;
use std::time::Duration;

/// HTTP 传输层 trait
///
/// 抽象 HTTP 客户端操作，支持：
/// - 普通请求
/// - 流式请求
/// - 超时控制
#[async_trait]
pub trait HttpTransport: Send + Sync + std::fmt::Debug {
    /// 发送 POST 请求并返回响应体
    async fn post_json(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<String>;

    /// 发送 POST 请求并返回字节流（用于 SSE）
    async fn post_stream(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<ByteStream>;

    /// 获取请求超时
    fn request_timeout(&self) -> Duration;

    /// 获取流式请求超时
    fn stream_timeout(&self) -> Duration;
}

/// 字节流类型
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;

/// 默认 HTTP 传输实现（使用 reqwest）
#[derive(Debug, Clone)]
pub struct DefaultHttpTransport {
    client: reqwest::Client,
    request_timeout: Duration,
    stream_timeout: Duration,
}

impl Default for DefaultHttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultHttpTransport {
    /// 创建新的默认传输
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            request_timeout: Duration::from_secs(120),
            stream_timeout: Duration::from_secs(600),
        }
    }

    /// 创建带自定义超时的传输
    pub fn with_timeouts(request_timeout: Duration, stream_timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::new(),
            request_timeout,
            stream_timeout,
        }
    }

    /// 构建请求
    fn build_request(
        &self,
        method: reqwest::Method,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> reqwest::RequestBuilder {
        let mut builder = self.client.request(method, url);
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        builder.body(body)
    }
}

#[async_trait]
impl HttpTransport for DefaultHttpTransport {
    async fn post_json(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<String> {
        let response = self
            .build_request(reqwest::Method::POST, url, headers, body)
            .timeout(self.request_timeout)
            .send()
            .await
            .map_err(|e| {
                keycompute_types::KeyComputeError::ProviderError(format!(
                    "HTTP request failed: {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(keycompute_types::KeyComputeError::ProviderError(format!(
                "HTTP error ({}): {}",
                status, error_text
            )));
        }

        response.text().await.map_err(|e| {
            keycompute_types::KeyComputeError::ProviderError(format!(
                "Failed to read response: {}",
                e
            ))
        })
    }

    async fn post_stream(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<ByteStream> {
        let response = self
            .build_request(reqwest::Method::POST, url, headers, body)
            .timeout(self.stream_timeout)
            .send()
            .await
            .map_err(|e| {
                keycompute_types::KeyComputeError::ProviderError(format!(
                    "HTTP stream request failed: {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(keycompute_types::KeyComputeError::ProviderError(format!(
                "HTTP error ({}): {}",
                status, error_text
            )));
        }

        // 转换字节流
        let stream = response.bytes_stream().map(|result| {
            result.map_err(|e| {
                keycompute_types::KeyComputeError::ProviderError(format!("Stream error: {}", e))
            })
        });

        Ok(Box::pin(stream))
    }

    fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    fn stream_timeout(&self) -> Duration {
        self.stream_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_transport_new() {
        let transport = DefaultHttpTransport::new();
        assert_eq!(transport.request_timeout(), Duration::from_secs(120));
        assert_eq!(transport.stream_timeout(), Duration::from_secs(600));
    }

    #[test]
    fn test_default_transport_with_timeouts() {
        let transport =
            DefaultHttpTransport::with_timeouts(Duration::from_secs(60), Duration::from_secs(300));
        assert_eq!(transport.request_timeout(), Duration::from_secs(60));
        assert_eq!(transport.stream_timeout(), Duration::from_secs(300));
    }
}
