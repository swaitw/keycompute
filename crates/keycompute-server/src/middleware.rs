//! 中间件
//!
//! 自定义中间件：认证、限流、可观测性等

use crate::{
    error::{ApiError, Result},
    extractors::AuthExtractor,
    state::AppState,
};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use keycompute_auth::Permission;
use keycompute_ratelimit::RateLimitKey;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 请求日志中间件
pub async fn request_logger(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    // 提前克隆 request_id，避免借用冲突
    let request_id = req
        .headers()
        .get("X-Request-ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Request started"
    );

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

/// CORS 中间件配置
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}

/// 追踪 ID 注入中间件
pub async fn trace_id_middleware(mut req: Request, next: Next) -> Response {
    // 如果没有 X-Request-ID，生成一个
    if !req.headers().contains_key("X-Request-ID") {
        let request_id = uuid::Uuid::new_v4().to_string();
        req.headers_mut()
            .insert("X-Request-ID", request_id.parse().unwrap());
    }
    next.run(req).await
}

/// 限流中间件
///
/// 基于用户/租户/API Key 进行请求限流
/// 注意：此中间件应在认证中间件之后运行，以获取真实的认证信息
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // 从请求头中提取认证信息
    let headers = req.headers();
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());

    // 如果没有认证头，直接放行（由认证中间件处理）
    let Some(auth_header) = auth_header else {
        return next.run(req).await;
    };

    // 解析 Bearer token
    let Some(token) = auth_header.strip_prefix("Bearer ") else {
        return next.run(req).await;
    };

    // 使用 AuthService 验证 token 获取真实的用户信息
    let rate_key = match state.auth.verify_api_key(token).await {
        Ok(auth_context) => {
            // 使用真实的 user_id, tenant_id, produce_ai_key_id 创建限流键
            RateLimitKey::new(
                auth_context.tenant_id,
                auth_context.user_id,
                auth_context.produce_ai_key_id,
            )
        }
        Err(_) => {
            // 认证失败，直接放行（由认证层处理错误）
            return next.run(req).await;
        }
    };

    // 检查限流
    match state.rate_limiter.check_and_record(&rate_key).await {
        Ok(()) => {
            // 限流检查通过，继续处理请求
            next.run(req).await
        }
        Err(keycompute_types::KeyComputeError::RateLimitExceeded) => {
            // 触发限流
            info!(
                "Rate limit exceeded for tenant: {}, user: {}",
                rate_key.tenant_id, rate_key.user_id
            );
            (
                StatusCode::TOO_MANY_REQUESTS,
                serde_json::json!({
                    "error": {
                        "message": "Rate limit exceeded. Please try again later.",
                        "type": "rate_limit_exceeded",
                        "code": "rate_limit_exceeded"
                    }
                })
                .to_string(),
            )
                .into_response()
        }
        Err(e) => {
            // 限流检查出错，记录错误但放行（避免阻塞正常请求）
            error!("Rate limit check error: {}", e);
            next.run(req).await
        }
    }
}

/// 权限检查中间件
///
/// 检查用户是否具有指定的权限
/// 管理员角色自动拥有所有权限
pub async fn require_permission(
    State(_state): State<AppState>,
    auth: AuthExtractor,
    req: Request,
    next: Next,
    required_permission: Permission,
) -> Result<Response> {
    use keycompute_auth::PermissionChecker;

    // 获取用户权限列表（这里简化处理，实际应从数据库或缓存获取）
    let user_permissions = if auth.is_admin() {
        vec![Permission::SystemAdmin]
    } else {
        vec![Permission::UseApi, Permission::ViewUsage]
    };

    if !PermissionChecker::check(&auth.role, &user_permissions, &required_permission) {
        return Err(ApiError::Auth(format!(
            "Permission denied: requires {:?}",
            required_permission
        )));
    }

    Ok(next.run(req).await)
}

/// 创建权限检查中间件层
///
/// 使用示例：
/// ```rust,ignore
/// // 在路由中使用权限中间件
/// Router::new()
///     .route("/api/v1/users", get(list_users))
///     .layer(from_fn_with_state(state.clone(), |state, auth, req, next| {
///         permission_middleware(state, auth, req, next, Permission::ManageUsers)
///     }))
/// ```
pub fn permission_middleware(
    permission: Permission,
) -> impl Fn(
    State<AppState>,
    AuthExtractor,
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send>>
+ Clone {
    move |state: State<AppState>, auth: AuthExtractor, req: Request, next: Next| {
        let perm = permission.clone();
        Box::pin(async move { require_permission(state, auth, req, next, perm).await })
    }
}

// ==================== Admin 认证中间件 ====================

/// Admin 认证中间件
///
/// 专为 Admin 路由设计，提供统一的权限保护：
/// 1. 验证请求是否携带有效的认证 Token
/// 2. 检查用户是否具有 Admin 角色
/// 3. 将认证信息注入请求扩展，供后续 Handler 使用
///
/// # 返回
/// - 成功：继续处理请求
/// - 401：未认证或认证失败
/// - 403：认证成功但非 Admin 角色
///
/// # 使用示例
/// ```rust,ignore
/// let admin_routes = Router::new()
///     .route("/api/v1/users", get(list_all_users))
///     .layer(from_fn_with_state(state.clone(), admin_auth_middleware));
/// ```
pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    // 1. 从请求头提取认证信息
    let headers = req.headers();
    let auth_header = match headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        Some(h) => h,
        None => {
            warn!("Admin route accessed without authentication");
            return (
                StatusCode::UNAUTHORIZED,
                serde_json::json!({
                    "error": {
                        "message": "Authentication required",
                        "type": "auth_required",
                        "code": "unauthorized"
                    }
                })
                .to_string(),
            )
                .into_response();
        }
    };

    // 2. 解析 Bearer token
    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => {
            warn!("Invalid authorization header format");
            return (
                StatusCode::UNAUTHORIZED,
                serde_json::json!({
                    "error": {
                        "message": "Invalid authorization format. Expected: Bearer <token>",
                        "type": "auth_invalid_format",
                        "code": "unauthorized"
                    }
                })
                .to_string(),
            )
                .into_response();
        }
    };

    // 3. 验证 token 并获取认证上下文
    let auth_context = match state.auth.verify_api_key(token).await {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!(error = %e, "Authentication failed for admin route");
            return (
                StatusCode::UNAUTHORIZED,
                serde_json::json!({
                    "error": {
                        "message": format!("Authentication failed: {}", e),
                        "type": "auth_failed",
                        "code": "unauthorized"
                    }
                })
                .to_string(),
            )
                .into_response();
        }
    };

    // 4. 检查 Admin 角色
    if auth_context.role != "admin" {
        warn!(
            user_id = %auth_context.user_id,
            role = %auth_context.role,
            "Non-admin user attempted to access admin route"
        );
        return (
            StatusCode::FORBIDDEN,
            serde_json::json!({
                "error": {
                    "message": "Admin permission required",
                    "type": "permission_denied",
                    "code": "forbidden"
                }
            })
            .to_string(),
        )
            .into_response();
    }

    // 5. 认证成功，注入认证信息到请求扩展
    // 创建 AuthExtractor 并存入请求扩展，供后续 Handler 使用
    let auth_extractor = AuthExtractor::from_auth_context(auth_context);
    req.extensions_mut().insert(auth_extractor);

    // 6. 继续处理请求
    info!("Admin authentication successful");
    next.run(req).await
}

/// 从请求扩展中提取 AuthExtractor
///
/// 用于在 Handler 中获取已由中间件验证的认证信息
///
/// # 使用示例
/// ```rust,ignore
/// pub async fn admin_handler(
///     Extension(auth): Extension<AuthExtractor>,
/// ) -> Result<Json<...>> {
///     // auth 已由 admin_auth_middleware 验证
///     Ok(Json(...))
/// }
/// ```
pub fn extract_auth_from_extensions(req: &Request) -> Option<AuthExtractor> {
    req.extensions().get::<AuthExtractor>().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;

    #[tokio::test]
    async fn test_cors_layer() {
        let cors = cors_layer();
        // 确保可以创建 CORS 层
        let _ = cors;
    }

    #[test]
    fn test_permission_middleware_creation() {
        // 测试权限中间件可以正确创建
        let _middleware = permission_middleware(Permission::SystemAdmin);
    }

    #[test]
    fn test_extract_auth_from_extensions_empty() {
        // 测试从空扩展中提取 AuthExtractor
        let req: Request<Body> = Request::new(Body::empty());
        let result = extract_auth_from_extensions(&req);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_auth_from_extensions_present() {
        // 测试从扩展中提取已注入的 AuthExtractor
        let mut req: Request<Body> = Request::new(Body::empty());
        let auth = AuthExtractor::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), "admin");
        req.extensions_mut().insert(auth.clone());

        let result = extract_auth_from_extensions(&req);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.is_admin());
    }
}
