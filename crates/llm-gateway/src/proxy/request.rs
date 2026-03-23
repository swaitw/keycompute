//! 代理请求
//!
//! 定义通过代理发送的请求结构

use bytes::Bytes;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

/// 代理请求
///
/// 封装通过 Internal HTTP Proxy 发送的请求
#[derive(Debug, Clone)]
pub struct ProxyRequest {
    /// 请求 URL
    pub url: String,
    /// HTTP 方法
    pub method: HttpMethod,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体（JSON）
    pub body: Option<serde_json::Value>,
    /// 请求 ID（用于追踪）
    pub request_id: Option<uuid::Uuid>,
    /// Provider 名称
    pub provider: Option<String>,
    /// 是否流式请求
    pub is_stream: bool,
    /// 自定义超时
    pub timeout: Option<Duration>,
}

/// HTTP 方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

impl Default for ProxyRequest {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body: None,
            request_id: None,
            provider: None,
            is_stream: false,
            timeout: None,
        }
    }
}

impl ProxyRequest {
    /// 创建新的代理请求
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// 创建 POST 请求
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::POST,
            ..Default::default()
        }
    }

    /// 创建 GET 请求
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::GET,
            ..Default::default()
        }
    }

    /// 设置请求头
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 设置 Authorization 头
    pub fn authorization(mut self, token: impl Into<String>) -> Self {
        self.headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", token.into()),
        );
        self
    }

    /// 设置 Content-Type 头
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.headers
            .insert("Content-Type".to_string(), content_type.into());
        self
    }

    /// 设置 JSON 请求体
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        self.body = Some(serde_json::to_value(body).unwrap_or(serde_json::Value::Null));
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self
    }

    /// 设置请求 ID
    pub fn request_id(mut self, id: uuid::Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// 设置 Provider 名称
    pub fn provider(mut self, name: impl Into<String>) -> Self {
        self.provider = Some(name.into());
        self
    }

    /// 设置为流式请求
    pub fn stream(mut self, is_stream: bool) -> Self {
        self.is_stream = is_stream;
        if is_stream {
            self.headers
                .insert("Accept".to_string(), "text/event-stream".to_string());
        }
        self
    }

    /// 设置超时
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 构建为 reqwest::RequestBuilder
    pub fn build(&self, client: &crate::proxy::HttpClient) -> reqwest::RequestBuilder {
        let mut builder = match self.method {
            HttpMethod::GET => client.get(&self.url),
            HttpMethod::POST => client.post(&self.url),
            HttpMethod::PUT => client.inner().put(&self.url),
            HttpMethod::DELETE => client.inner().delete(&self.url),
            HttpMethod::PATCH => client.inner().patch(&self.url),
        };

        // 添加请求头
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        // 添加请求体
        if let Some(body) = &self.body {
            builder = builder.json(body);
        }

        // 添加追踪头
        if let Some(id) = self.request_id {
            builder = builder.header("X-Request-ID", id.to_string());
        }
        if let Some(provider) = &self.provider {
            builder = builder.header("X-Provider", provider);
        }

        // 设置超时
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        } else if self.is_stream {
            // 流式请求使用更长的超时
            builder = builder.timeout(client.config().stream_timeout);
        }

        builder
    }
}

/// 代理响应
#[derive(Debug)]
pub struct ProxyResponse {
    /// HTTP 状态码
    pub status: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体
    pub body: Option<Bytes>,
}

impl ProxyResponse {
    /// 检查是否成功
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// 获取 JSON 响应体
    pub fn json<T: for<'de> serde::Deserialize<'de>>(&self) -> Option<T> {
        self.body
            .as_ref()
            .and_then(|b| serde_json::from_slice(b).ok())
    }

    /// 获取文本响应体
    pub fn text(&self) -> Option<String> {
        self.body
            .as_ref()
            .and_then(|b| String::from_utf8(b.to_vec()).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_request_new() {
        let req = ProxyRequest::new("https://api.example.com");
        assert_eq!(req.url, "https://api.example.com");
        assert_eq!(req.method, HttpMethod::POST);
    }

    #[test]
    fn test_proxy_request_builder() {
        let req = ProxyRequest::post("https://api.example.com/v1/chat")
            .authorization("sk-test")
            .content_type("application/json")
            .provider("openai")
            .stream(true);

        assert_eq!(
            req.headers.get("Authorization"),
            Some(&"Bearer sk-test".to_string())
        );
        assert_eq!(
            req.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            req.headers.get("Accept"),
            Some(&"text/event-stream".to_string())
        );
        assert!(req.is_stream);
    }

    #[test]
    fn test_proxy_request_json() {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let req = ProxyRequest::post("https://api.example.com/v1/chat").json(&body);

        assert!(req.body.is_some());
        assert_eq!(
            req.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_proxy_response_is_success() {
        let resp = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: None,
        };
        assert!(resp.is_success());

        let resp = ProxyResponse {
            status: 404,
            headers: HashMap::new(),
            body: None,
        };
        assert!(!resp.is_success());
    }
}
