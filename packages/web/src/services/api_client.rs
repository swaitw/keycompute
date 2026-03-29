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
