use std::sync::LazyLock;

use client_api::error::{ClientError, Result};
use client_api::{ApiClient, ClientConfig};

use crate::stores::auth_store::AuthStore;

/// 全局单例 API 客户端
/// ApiClient 内部持有 Arc，Clone 只是增加引用计数，开销极低
static CLIENT: LazyLock<ApiClient> = LazyLock::new(|| {
    let base_url = option_env!("API_BASE_URL")
        .unwrap_or("http://localhost:8080")
        .to_string();
    let config = ClientConfig::new(base_url);
    ApiClient::new(config).expect("Failed to create API client")
});

/// 获取全局 API 客户端实例（廉价克隆，仅增加 Arc 引用计数）
pub fn get_client() -> ApiClient {
    CLIENT.clone()
}

/// Token 自动刺新封装器
///
/// 在 service 层调用任意异步 API 时，若返回 `ClientError::Unauthorized`，
/// 则使用 refresh_token 自动钠新 access_token 并重试一次。
///
/// # 示例
/// ```rust
/// let result = with_auto_refresh(auth_store, |token| async move {
///     some_service::fetch(&token).await
/// }).await;
/// ```
pub async fn with_auto_refresh<F, Fut, T>(mut auth_store: AuthStore, f: F) -> Result<T>
where
    F: Fn(String) -> Fut + Clone,
    Fut: std::future::Future<Output = Result<T>>,
{
    let token = auth_store.token().unwrap_or_default();
    match f(token).await {
        Err(ClientError::Unauthorized(_)) => {
            // 尝试用 refresh_token 获取新 access_token
            let refresh = match auth_store.refresh_token() {
                Some(r) => r,
                None => return Err(ClientError::Unauthorized("no refresh token".to_string())),
            };
            match super::auth_service::refresh_token(&refresh).await {
                Ok(resp) => {
                    auth_store.login(resp.access_token.clone(), resp.refresh_token.clone());
                    // 用新 token 重试请求
                    f(resp.access_token).await
                }
                Err(e) => {
                    // 刺新失败，强制登出
                    auth_store.logout();
                    Err(e)
                }
            }
        }
        other => other,
    }
}

/// 将 ClientError 转为用户友好的中文提示文本
///
/// 在 UI 层展示错误时调用，避免直接折射原始英文错误字符串给用户。
#[allow(dead_code)]
pub fn localize_error(err: &client_api::error::ClientError) -> String {
    use client_api::error::ClientError;
    match err {
        ClientError::Unauthorized(_) => "登录已过期，请重新登录".to_string(),
        ClientError::Forbidden(_) => "权限不足，无法执行此操作".to_string(),
        ClientError::NotFound(_) => "资源不存在或已被删除".to_string(),
        ClientError::RateLimited(_) => "请求过于频繁，请稍候再试".to_string(),
        ClientError::Network(_) => "网络连接失败，请检查网络设置".to_string(),
        ClientError::ServerError(_) => "服务器内部错误，请稍候重试".to_string(),
        ClientError::ServiceUnavailable(_) => "服务暂时不可用，请稍候再试".to_string(),
        ClientError::Serialization(_) | ClientError::InvalidResponse(_) => {
            "数据解析失败，请刷新页面".to_string()
        }
        ClientError::Config(msg) => format!("配置错误：{}", msg),
        ClientError::Http(msg) => {
            // 尝试提取状态码后的消息部分
            if msg.contains("400") {
                "请求参数错误，请检查输入".to_string()
            } else if msg.contains("409") {
                "数据冲突，该资源可能已存在".to_string()
            } else {
                "请求失败，请稍候重试".to_string()
            }
        }
        ClientError::Other(msg) => msg.clone(),
    }
}
