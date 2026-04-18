//! 用户注册服务
//!
//! 提供用户注册功能，包括邮箱验证流程

use crate::jwt::JwtValidator;
use crate::password::{EmailValidator, PasswordHasher, PasswordValidator};
use chrono::{Duration, Utc};
use keycompute_db::{
    CreateUserCredentialRequest, CreateUserRequest, EmailVerification, Tenant, User, UserCredential,
};
use keycompute_emailserver::EmailService;
use keycompute_types::{KeyComputeError, Result, UserRole};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// 注册请求
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    /// 邮箱
    pub email: String,
    /// 密码
    pub password: String,
    /// 用户名（可选）
    pub name: Option<String>,
    /// 租户 Slug（可选，用于多租户注册）
    pub tenant_slug: Option<String>,
}

/// 注册响应
#[derive(Debug, Clone, Serialize)]
pub struct RegisterResponse {
    /// 用户 ID
    pub user_id: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 邮箱
    pub email: String,
    /// 消息
    pub message: String,
}

/// 注册服务
#[derive(Clone)]
pub struct RegistrationService {
    /// 数据库连接池
    pool: Arc<PgPool>,
    /// 密码哈希器
    password_hasher: PasswordHasher,
    /// 密码验证器
    password_validator: PasswordValidator,
    /// 邮箱验证器
    email_validator: EmailValidator,
    /// JWT 验证器（用于生成令牌）
    jwt_validator: Option<JwtValidator>,
    /// 邮件服务
    email_service: Option<EmailService>,
    /// 邮箱验证令牌有效期（小时）
    email_verification_expiry_hours: i64,
    /// 是否要求邮箱验证后才能登录
    require_email_verification: bool,
}

impl std::fmt::Debug for RegistrationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistrationService")
            .field(
                "email_verification_expiry_hours",
                &self.email_verification_expiry_hours,
            )
            .field(
                "require_email_verification",
                &self.require_email_verification,
            )
            .finish()
    }
}

impl RegistrationService {
    /// 创建新的注册服务
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            password_hasher: PasswordHasher::new(),
            password_validator: PasswordValidator::new(),
            email_validator: EmailValidator::new(),
            jwt_validator: None,
            email_service: None,
            email_verification_expiry_hours: 24,
            require_email_verification: true,
        }
    }

    /// 设置 JWT 验证器
    pub fn with_jwt_validator(mut self, jwt_validator: JwtValidator) -> Self {
        self.jwt_validator = Some(jwt_validator);
        self
    }

    /// 设置邮件服务
    pub fn with_email_service(mut self, email_service: EmailService) -> Self {
        self.email_service = Some(email_service);
        self
    }

    /// 设置邮箱验证令牌有效期
    pub fn with_verification_expiry(mut self, hours: i64) -> Self {
        self.email_verification_expiry_hours = hours;
        self
    }

    /// 设置是否要求邮箱验证
    pub fn with_email_verification_required(mut self, required: bool) -> Self {
        self.require_email_verification = required;
        self
    }

    /// 用户注册
    ///
    /// # 流程
    /// 1. 验证邮箱格式
    /// 2. 验证密码强度
    /// 3. 检查邮箱是否已被注册
    /// 4. 获取或创建租户
    /// 5. 创建用户
    /// 6. 创建密码凭证
    /// 7. 生成邮箱验证令牌
    /// 8. 发送验证邮件（需要调用方处理）
    pub async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse> {
        // 1. 规范化并验证邮箱
        let email = self.email_validator.normalize(&req.email);
        self.email_validator.validate(&email)?;

        // 2. 验证密码强度
        self.password_validator.validate(&req.password)?;

        // 3. 检查邮箱是否已被注册
        if User::find_by_email(&self.pool, &email)
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to check email existence: {}", e))
            })?
            .is_some()
        {
            return Err(KeyComputeError::AuthError("该邮箱已被注册".to_string()));
        }

        // 4. 获取或创建默认租户
        let tenant = self.get_or_create_default_tenant(&req.tenant_slug).await?;

        // 5. 创建用户
        let user = User::create(
            &self.pool,
            &CreateUserRequest {
                tenant_id: tenant.id,
                email: email.clone(),
                name: req.name.clone(),
                role: Some(UserRole::User),
            },
        )
        .await
        .map_err(|e| KeyComputeError::DatabaseError(format!("Failed to create user: {}", e)))?;

        // 6. 哈希密码并创建凭证
        let password_hash = self.password_hasher.hash(&req.password)?;
        let _credential = UserCredential::create(
            &self.pool,
            &CreateUserCredentialRequest {
                user_id: user.id,
                password_hash,
            },
        )
        .await
        .map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to create credential: {}", e))
        })?;

        // 7. 生成邮箱验证令牌
        let verification_token = self.generate_verification_token();
        let expires_at = Utc::now() + Duration::hours(self.email_verification_expiry_hours);

        EmailVerification::create(
            &self.pool,
            &keycompute_db::CreateEmailVerificationRequest {
                user_id: user.id,
                email: email.clone(),
                token: verification_token.clone(),
                expires_at,
            },
        )
        .await
        .map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to create email verification: {}", e))
        })?;

        // 8. 发送验证邮件
        if let Some(email_service) = &self.email_service {
            if let Err(e) = email_service
                .send_verification_email(&email, &verification_token)
                .await
            {
                tracing::error!(
                    user_id = %user.id,
                    email = %email,
                    error = %e,
                    "Failed to send verification email"
                );
                // 邮件发送失败不阻塞注册流程，用户可以重发验证邮件
            }
        } else {
            tracing::warn!(
                user_id = %user.id,
                email = %email,
                "Email service not configured, verification email not sent"
            );
        }

        tracing::info!(
            user_id = %user.id,
            tenant_id = %tenant.id,
            email = %email,
            "User registered successfully"
        );

        Ok(RegisterResponse {
            user_id: user.id,
            tenant_id: tenant.id,
            email: email.clone(),
            message: if self.require_email_verification {
                "注册成功，请查收验证邮件完成邮箱验证".to_string()
            } else {
                "注册成功".to_string()
            },
        })
    }

    /// 验证邮箱
    ///
    /// # Arguments
    /// * `token` - 验证令牌
    ///
    /// # Returns
    /// 验证成功返回用户 ID
    pub async fn verify_email(&self, token: &str) -> Result<Uuid> {
        // 查找验证记录
        let verification = EmailVerification::find_by_token(&self.pool, token)
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to find verification token: {}", e))
            })?
            .ok_or_else(|| KeyComputeError::AuthError("无效的验证链接".to_string()))?;

        // 检查是否有效
        if !verification.is_valid() {
            if verification.used {
                return Err(KeyComputeError::AuthError("该验证链接已使用".to_string()));
            }
            return Err(KeyComputeError::AuthError(
                "验证链接已过期，请重新发送".to_string(),
            ));
        }

        // 标记验证令牌已使用
        verification.mark_used(&self.pool).await.map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to mark verification as used: {}", e))
        })?;

        // 更新用户凭证的邮箱验证状态
        let credential = UserCredential::find_by_user_id(&self.pool, verification.user_id)
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to find credential: {}", e))
            })?
            .ok_or_else(|| KeyComputeError::AuthError("用户凭证不存在".to_string()))?;

        credential
            .update(
                &self.pool,
                &keycompute_db::UpdateUserCredentialRequest {
                    email_verified: Some(true),
                    email_verified_at: Some(Utc::now()),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to update credential: {}", e))
            })?;

        // 发送欢迎邮件（可选）
        if let Some(email_service) = &self.email_service
            && let Ok(Some(user)) = User::find_by_id(&self.pool, verification.user_id).await
            && let Err(e) = email_service
                .send_welcome_email(&verification.email, user.name.as_deref())
                .await
        {
            tracing::warn!(
                user_id = %verification.user_id,
                error = %e,
                "Failed to send welcome email"
            );
        }

        tracing::info!(
            user_id = %verification.user_id,
            "Email verified successfully"
        );

        Ok(verification.user_id)
    }

    /// 重新发送验证邮件
    ///
    /// # Arguments
    /// * `email` - 用户邮箱
    pub async fn resend_verification(&self, email: &str) -> Result<String> {
        let email = self.email_validator.normalize(email);

        // 查找用户
        let user = User::find_by_email(&self.pool, &email)
            .await
            .map_err(|e| KeyComputeError::DatabaseError(format!("Failed to find user: {}", e)))?
            .ok_or_else(|| KeyComputeError::AuthError("用户不存在".to_string()))?;

        // 检查是否已验证
        let credential = UserCredential::find_by_user_id(&self.pool, user.id)
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to find credential: {}", e))
            })?
            .ok_or_else(|| KeyComputeError::AuthError("用户凭证不存在".to_string()))?;

        if credential.email_verified {
            return Err(KeyComputeError::AuthError("邮箱已验证".to_string()));
        }

        // 生成新的验证令牌
        let verification_token = self.generate_verification_token();
        let expires_at = Utc::now() + Duration::hours(self.email_verification_expiry_hours);

        EmailVerification::create(
            &self.pool,
            &keycompute_db::CreateEmailVerificationRequest {
                user_id: user.id,
                email: email.clone(),
                token: verification_token.clone(),
                expires_at,
            },
        )
        .await
        .map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to create email verification: {}", e))
        })?;

        // 发送验证邮件
        if let Some(email_service) = &self.email_service
            && let Err(e) = email_service
                .send_verification_email(&email, &verification_token)
                .await
        {
            tracing::error!(
                user_id = %user.id,
                email = %email,
                error = %e,
                "Failed to resend verification email"
            );
            return Err(KeyComputeError::AuthError(
                "发送验证邮件失败，请稍后重试".to_string(),
            ));
        }

        tracing::info!(
            user_id = %user.id,
            email = %email,
            "Verification email resent"
        );

        Ok(verification_token)
    }

    /// 获取或创建默认租户
    async fn get_or_create_default_tenant(&self, tenant_slug: &Option<String>) -> Result<Tenant> {
        if let Some(slug) = tenant_slug {
            // 查找指定租户
            let tenant = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE slug = $1")
                .bind(slug)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| {
                    KeyComputeError::DatabaseError(format!("Failed to find tenant: {}", e))
                })?
                .ok_or_else(|| KeyComputeError::AuthError(format!("租户不存在: {}", slug)))?;

            return Ok(tenant);
        }

        // 使用或创建默认租户
        let default_slug = "default";

        if let Some(tenant) = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE slug = $1")
            .bind(default_slug)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to find default tenant: {}", e))
            })?
        {
            return Ok(tenant);
        }

        // 创建默认租户
        let tenant = Tenant::create(
            &self.pool,
            &keycompute_db::CreateTenantRequest {
                name: "Default Tenant".to_string(),
                slug: default_slug.to_string(),
                description: Some("Default tenant for new users".to_string()),
                default_rpm_limit: None,
                default_tpm_limit: None,
                distribution_enabled: None,
            },
        )
        .await
        .map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to create default tenant: {}", e))
        })?;

        Ok(tenant)
    }

    /// 生成安全的验证令牌
    fn generate_verification_token(&self) -> String {
        let mut rng = rand::thread_rng();
        let token: String = (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..62);
                if idx < 10 {
                    (b'0' + idx as u8) as char
                } else if idx < 36 {
                    (b'a' + idx as u8 - 10) as char
                } else {
                    (b'A' + idx as u8 - 36) as char
                }
            })
            .collect();
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_verification_token() {
        // 测试令牌生成逻辑，不需要数据库连接
        let mut rng = rand::thread_rng();
        let token: String = (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..62);
                if idx < 10 {
                    (b'0' + idx as u8) as char
                } else if idx < 36 {
                    (b'a' + idx as u8 - 10) as char
                } else {
                    (b'A' + idx as u8 - 36) as char
                }
            })
            .collect();

        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_alphanumeric()));
    }
}
