//! 错误处理
//!
//! 定义 API 错误类型和响应格式

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::fmt;

/// API 错误类型
#[derive(Debug)]
pub enum ApiError {
    /// 认证错误
    Auth(String),
    /// 限流错误
    RateLimit(String),
    /// 路由错误
    Routing(String),
    /// Provider 错误
    Provider(String),
    /// 配置错误
    Config(String),
    /// 内部错误
    Internal(String),
    /// 请求参数错误
    BadRequest(String),
    /// 资源未找到
    NotFound(String),
    /// 权限拒绝
    Forbidden(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Auth(msg) => write!(f, "Authentication error: {}", msg),
            ApiError::RateLimit(msg) => write!(f, "Rate limit error: {}", msg),
            ApiError::Routing(msg) => write!(f, "Routing error: {}", msg),
            ApiError::Provider(msg) => write!(f, "Provider error: {}", msg),
            ApiError::Config(msg) => write!(f, "Config error: {}", msg),
            ApiError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            ApiError::RateLimit(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone()),
            ApiError::Routing(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            ApiError::Provider(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            ApiError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": error_type(&self),
                "code": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}

fn error_type(error: &ApiError) -> &'static str {
    match error {
        ApiError::Auth(_) => "authentication_error",
        ApiError::RateLimit(_) => "rate_limit_error",
        ApiError::Routing(_) => "routing_error",
        ApiError::Provider(_) => "provider_error",
        ApiError::Config(_) => "config_error",
        ApiError::Internal(_) => "internal_error",
        ApiError::BadRequest(_) => "bad_request_error",
        ApiError::NotFound(_) => "not_found_error",
        ApiError::Forbidden(_) => "forbidden_error",
    }
}

/// API 结果类型
pub type Result<T> = std::result::Result<T, ApiError>;

/// 从 keycompute-types 错误转换
impl From<keycompute_types::KeyComputeError> for ApiError {
    fn from(err: keycompute_types::KeyComputeError) -> Self {
        match err {
            keycompute_types::KeyComputeError::AuthError(msg) => ApiError::Auth(msg),
            keycompute_types::KeyComputeError::RateLimitExceeded => {
                ApiError::RateLimit("Rate limit exceeded".to_string())
            }
            keycompute_types::KeyComputeError::RoutingFailed => {
                ApiError::Routing("No available provider".to_string())
            }
            keycompute_types::KeyComputeError::ProviderError(msg) => ApiError::Provider(msg),
            keycompute_types::KeyComputeError::Internal(msg) => ApiError::Internal(msg),
            keycompute_types::KeyComputeError::ValidationError(msg) => ApiError::BadRequest(msg),
            keycompute_types::KeyComputeError::NotFound(msg) => ApiError::NotFound(msg),
            _ => ApiError::Internal(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let err = ApiError::Auth("Invalid key".to_string());
        assert!(err.to_string().contains("Authentication error"));
    }

    #[test]
    fn test_error_type() {
        assert_eq!(
            error_type(&ApiError::Auth("test".to_string())),
            "authentication_error"
        );
        assert_eq!(
            error_type(&ApiError::RateLimit("test".to_string())),
            "rate_limit_error"
        );
    }
}
