//! 邮箱验证模型
//!
//! 管理用户邮箱验证流程

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 邮箱验证记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EmailVerification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 创建邮箱验证请求
#[derive(Debug, Clone)]
pub struct CreateEmailVerificationRequest {
    pub user_id: Uuid,
    pub email: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

impl EmailVerification {
    /// 创建新验证记录
    pub async fn create(
        pool: &sqlx::PgPool,
        req: &CreateEmailVerificationRequest,
    ) -> Result<EmailVerification, sqlx::Error> {
        // 使用 ON CONFLICT 处理重复记录
        let verification = sqlx::query_as::<_, EmailVerification>(
            r#"
            INSERT INTO email_verifications (user_id, email, token, expires_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, email) DO UPDATE SET
                token = EXCLUDED.token,
                expires_at = EXCLUDED.expires_at,
                used = FALSE,
                used_at = NULL
            RETURNING *
            "#,
        )
        .bind(req.user_id)
        .bind(&req.email)
        .bind(&req.token)
        .bind(req.expires_at)
        .fetch_one(pool)
        .await?;

        Ok(verification)
    }

    /// 根据 ID 查找
    pub async fn find_by_id(
        pool: &sqlx::PgPool,
        id: Uuid,
    ) -> Result<Option<EmailVerification>, sqlx::Error> {
        let verification = sqlx::query_as::<_, EmailVerification>(
            "SELECT * FROM email_verifications WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(verification)
    }

    /// 根据令牌查找
    pub async fn find_by_token(
        pool: &sqlx::PgPool,
        token: &str,
    ) -> Result<Option<EmailVerification>, sqlx::Error> {
        let verification = sqlx::query_as::<_, EmailVerification>(
            "SELECT * FROM email_verifications WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await?;

        Ok(verification)
    }

    /// 根据用户 ID 查找最新的验证记录
    pub async fn find_latest_by_user(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<Option<EmailVerification>, sqlx::Error> {
        let verification = sqlx::query_as::<_, EmailVerification>(
            "SELECT * FROM email_verifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(verification)
    }

    /// 标记为已使用
    pub async fn mark_used(&self, pool: &sqlx::PgPool) -> Result<EmailVerification, sqlx::Error> {
        let verification = sqlx::query_as::<_, EmailVerification>(
            r#"
            UPDATE email_verifications
            SET used = TRUE,
                used_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(verification)
    }

    /// 删除验证记录
    pub async fn delete(&self, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM email_verifications WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 检查令牌是否有效（未使用且未过期）
    pub fn is_valid(&self) -> bool {
        !self.used && self.expires_at > Utc::now()
    }

    /// 检查令牌是否过期
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_verification_is_valid() {
        let verification = EmailVerification {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token: "token123".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            used: false,
            used_at: None,
            created_at: Utc::now(),
        };

        assert!(verification.is_valid());
        assert!(!verification.is_expired());
    }

    #[test]
    fn test_email_verification_used() {
        let verification = EmailVerification {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token: "token123".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            used: true,
            used_at: Some(Utc::now()),
            created_at: Utc::now(),
        };

        assert!(!verification.is_valid());
    }

    #[test]
    fn test_email_verification_expired() {
        let verification = EmailVerification {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token: "token123".to_string(),
            expires_at: Utc::now() - chrono::Duration::hours(1),
            used: false,
            used_at: None,
            created_at: Utc::now(),
        };

        assert!(!verification.is_valid());
        assert!(verification.is_expired());
    }
}
