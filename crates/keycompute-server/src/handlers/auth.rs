//! 认证处理器
//!
//! 处理用户注册、登录、邮箱验证、密码重置等认证相关的 HTTP 请求

use crate::{
    error::{ApiError, Result},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use keycompute_auth::{
    LoginRequest, PasswordResetService, RegisterRequest, RegistrationService,
    RequestPasswordResetRequest, ResetPasswordRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// 辅助函数
// ============================================================================

/// 从请求头中提取客户端 IP 地址
///
/// 优先级：X-Forwarded-For > X-Real-IP
///
/// # 参数
/// - `headers`: HTTP 请求头
///
/// # 返回
/// - 提取到的 IP 地址字符串，如果无法提取则返回 None
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // 1. 尝试从 X-Forwarded-For 获取（反向代理场景）
    // X-Forwarded-For 格式：client, proxy1, proxy2
    // 我们需要第一个（最左边的）IP
    if let Some(forwarded) = headers.get("x-forwarded-for")
        && let Ok(value) = forwarded.to_str()
        && let Some(client_ip) = value.split(',').next()
    {
        let ip = client_ip.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }

    // 2. 尝试从 X-Real-IP 获取（Nginx 常用）
    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(value) = real_ip.to_str()
    {
        let ip = value.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }

    // 3. 尝试从 CF-Connecting-IP 获取（Cloudflare 专用）
    if let Some(cf_ip) = headers.get("cf-connecting-ip")
        && let Ok(value) = cf_ip.to_str()
    {
        let ip = value.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }

    None
}

// ============================================================================
// 请求/响应类型
// ============================================================================

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequestJson {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub tenant_slug: Option<String>,
    /// 推荐码（推荐人的用户 ID）
    pub referral_code: Option<String>,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequestJson {
    pub email: String,
    pub password: String,
}

/// 忘记密码请求
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequestJson {
    pub email: String,
}

/// 重置密码请求
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequestJson {
    pub token: String,
    pub new_password: String,
}

/// 通用消息响应
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

/// 验证令牌响应
#[derive(Debug, Serialize)]
pub struct VerifyTokenResponse {
    pub valid: bool,
    pub user_id: Option<String>,
}

// ============================================================================
// 处理器函数
// ============================================================================

/// 用户注册
///
/// POST /auth/register
pub async fn register_handler(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequestJson>,
) -> Result<impl IntoResponse> {
    use keycompute_db::models::system_setting::setting_keys;

    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    // 检查是否允许注册
    let allow_registration =
        keycompute_db::SystemSetting::get_bool(pool, setting_keys::ALLOW_REGISTRATION, true).await;

    if !allow_registration {
        return Err(ApiError::Forbidden(
            "New user registration is currently disabled".to_string(),
        ));
    }

    // 获取默认用户配额
    let default_quota =
        keycompute_db::SystemSetting::get_decimal(pool, setting_keys::DEFAULT_USER_QUOTA, 10.0)
            .await;

    let register_req = RegisterRequest {
        email: req.email,
        password: req.password,
        name: req.name,
        tenant_slug: req.tenant_slug,
    };

    let service = RegistrationService::new(Arc::clone(pool))
        .with_email_service((*state.email_service).clone());
    let response = service
        .register(&register_req)
        .await
        .map_err(|e| ApiError::Auth(format!("Registration failed: {}", e)))?;

    // 为新用户设置初始余额（默认配额）
    if default_quota > 0.0 {
        if let Err(e) =
            initialize_user_balance(pool, response.user_id, response.tenant_id, default_quota).await
        {
            tracing::warn!(
                user_id = %response.user_id,
                quota = default_quota,
                error = %e,
                "Failed to initialize user balance"
            );
        } else {
            tracing::info!(
                user_id = %response.user_id,
                quota = default_quota,
                "User balance initialized with default quota"
            );
        }
    }

    // 处理推荐关系
    if let Some(ref referral_code) = req.referral_code
        && let Ok(level1_referrer_id) = Uuid::parse_str(referral_code)
    {
        // 查找一级推荐人的推荐人（二级推荐人）
        let level2_referrer_id =
            keycompute_db::UserReferral::find_by_user(pool, level1_referrer_id)
                .await
                .ok()
                .flatten()
                .and_then(|r| r.level1_referrer_id);

        // 创建推荐关系
        let referral_req = keycompute_db::CreateUserReferralRequest {
            user_id: response.user_id,
            level1_referrer_id: Some(level1_referrer_id),
            level2_referrer_id,
            source: Some("referral_code".to_string()),
        };

        if let Err(e) = keycompute_db::UserReferral::create(pool, &referral_req).await {
            tracing::warn!(
                user_id = %response.user_id,
                referrer_id = %level1_referrer_id,
                error = %e,
                "Failed to create referral relationship"
            );
        } else {
            tracing::info!(
                user_id = %response.user_id,
                level1_referrer = %level1_referrer_id,
                level2_referrer = ?level2_referrer_id,
                "Referral relationship created"
            );
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "user_id": response.user_id.to_string(),
            "tenant_id": response.tenant_id.to_string(),
            "email": response.email,
            "message": response.message
        })),
    ))
}

/// 用户登录
///
/// POST /auth/login
pub async fn login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequestJson>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let jwt_validator = state
        .auth
        .get_jwt_validator()
        .ok_or_else(|| ApiError::Internal("JWT not configured".into()))?
        .clone();

    let login_req = LoginRequest {
        email: req.email,
        password: req.password,
        client_ip: extract_client_ip(&headers),
    };

    let service = keycompute_auth::LoginService::new(Arc::clone(pool), jwt_validator);
    let response = service.login(&login_req).await.map_err(|e| match e {
        keycompute_types::KeyComputeError::AuthError(msg) => ApiError::Auth(msg),
        _ => ApiError::Internal(format!("Login failed: {}", e)),
    })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "user_id": response.user_id.to_string(),
            "tenant_id": response.tenant_id.to_string(),
            "email": response.email,
            "role": response.role,
            "access_token": response.jwt_token,
            "token_type": "Bearer",
            "expires_in": response.expires_in
        })),
    ))
}

/// 邮箱验证
///
/// GET /auth/verify-email/{token}
pub async fn verify_email_handler(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let service = RegistrationService::new(Arc::clone(pool))
        .with_email_service((*state.email_service).clone());
    let user_id = service
        .verify_email(&token)
        .await
        .map_err(|e| ApiError::Auth(format!("Email verification failed: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Email verified successfully",
            "user_id": user_id.to_string()
        })),
    ))
}

/// 忘记密码
///
/// POST /auth/forgot-password
pub async fn forgot_password_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ForgotPasswordRequestJson>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let service = PasswordResetService::new(Arc::clone(pool))
        .with_email_service((*state.email_service).clone());

    // 无论邮箱是否存在都返回成功（防止邮箱枚举攻击）
    service
        .request_reset(&RequestPasswordResetRequest {
            email: req.email,
            client_ip: extract_client_ip(&headers),
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Password reset request failed: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(MessageResponse {
            message: "If the email exists, a reset link has been sent.".to_string(),
        }),
    ))
}

/// 重置密码
///
/// POST /auth/reset-password
pub async fn reset_password_handler(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequestJson>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let service = PasswordResetService::new(Arc::clone(pool));

    let reset_req = ResetPasswordRequest {
        token: req.token,
        new_password: req.new_password,
    };

    let user_id = service
        .reset_password(&reset_req)
        .await
        .map_err(|e| match e {
            keycompute_types::KeyComputeError::AuthError(msg) => ApiError::Auth(msg),
            keycompute_types::KeyComputeError::ValidationError(msg) => ApiError::BadRequest(msg),
            _ => ApiError::Internal(format!("Password reset failed: {}", e)),
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Password reset successfully",
            "user_id": user_id.to_string()
        })),
    ))
}

/// 验证重置令牌
///
/// GET /auth/verify-reset-token/:token
pub async fn verify_reset_token_handler(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let service = PasswordResetService::new(Arc::clone(pool));
    let valid = service
        .verify_token(&token)
        .await
        .map_err(|e| ApiError::Internal(format!("Token verification failed: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(VerifyTokenResponse {
            valid,
            user_id: None,
        }),
    ))
}

/// 刷新 Token
///
/// POST /auth/refresh-token
pub async fn refresh_token_handler(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequestJson>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let jwt_validator = state
        .auth
        .get_jwt_validator()
        .ok_or_else(|| ApiError::Internal("JWT not configured".into()))?
        .clone();

    let service = keycompute_auth::LoginService::new(Arc::clone(pool), jwt_validator);
    let response = service
        .refresh_token(&req.token)
        .await
        .map_err(|e| ApiError::Auth(format!("Token refresh failed: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "user_id": response.user_id.to_string(),
            "tenant_id": response.tenant_id.to_string(),
            "email": response.email,
            "role": response.role,
            "access_token": response.jwt_token,
            "token_type": "Bearer",
            "expires_in": response.expires_in
        })),
    ))
}

/// 刷新 Token 请求
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequestJson {
    pub token: String,
}

/// 重新发送验证邮件
///
/// POST /auth/resend-verification
pub async fn resend_verification_handler(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequestJson>,
) -> Result<impl IntoResponse> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".into()))?;

    let service = RegistrationService::new(Arc::clone(pool))
        .with_email_service((*state.email_service).clone());
    service
        .resend_verification(&req.email)
        .await
        .map_err(|e| match e {
            keycompute_types::KeyComputeError::AuthError(msg) => ApiError::Auth(msg),
            _ => ApiError::Internal(format!("Resend verification failed: {}", e)),
        })?;

    Ok((
        StatusCode::OK,
        Json(MessageResponse {
            message:
                "If the email exists and is not verified, a new verification email has been sent."
                    .to_string(),
        }),
    ))
}

// ==================== 辅助函数 ====================

/// 初始化用户余额
///
/// 为新用户设置初始余额（默认配额）
async fn initialize_user_balance(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    initial_balance: f64,
) -> std::result::Result<(), sqlx::Error> {
    use rust_decimal::Decimal;

    let amount = Decimal::from_f64_retain(initial_balance).unwrap_or(Decimal::ZERO);

    // 创建或更新余额记录
    sqlx::query(
        r#"
        INSERT INTO user_balances (tenant_id, user_id, available_balance, total_recharged)
        VALUES ($1, $2, $3, $3)
        ON CONFLICT (user_id) DO UPDATE SET
            available_balance = user_balances.available_balance + $3,
            total_recharged = user_balances.total_recharged + $3,
            updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(amount)
    .execute(pool)
    .await?;

    // 记录交易
    sqlx::query(
        r#"
        INSERT INTO balance_transactions (
            tenant_id, user_id, transaction_type, amount, balance_before, balance_after, description
        )
        VALUES ($1, $2, 'recharge', $3, 0, $3, 'Initial quota from system')
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(amount)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_request_json() {
        let json = RegisterRequestJson {
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
            name: Some("Test User".to_string()),
            tenant_slug: None,
            referral_code: None,
        };

        assert_eq!(json.email, "test@example.com");
    }

    #[test]
    fn test_login_request_json() {
        let json = LoginRequestJson {
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
        };

        assert_eq!(json.email, "test@example.com");
    }

    #[test]
    fn test_message_response() {
        let resp = MessageResponse {
            message: "Success".to_string(),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Success"));
    }
}
