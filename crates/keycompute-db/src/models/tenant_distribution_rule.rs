use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 租户分销规则模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TenantDistributionRule {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub beneficiary_id: Uuid,
    pub share_ratio: BigDecimal,
    pub priority: i32,
    pub enabled: bool,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建分销规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDistributionRuleRequest {
    pub tenant_id: Uuid,
    pub beneficiary_id: Uuid,
    pub share_ratio: BigDecimal,
    pub priority: Option<i32>,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_until: Option<DateTime<Utc>>,
}

/// 更新分销规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDistributionRuleRequest {
    pub share_ratio: Option<BigDecimal>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub effective_until: Option<DateTime<Utc>>,
}

impl TenantDistributionRule {
    /// 创建新分销规则
    pub async fn create(
        pool: &sqlx::PgPool,
        req: &CreateDistributionRuleRequest,
    ) -> Result<TenantDistributionRule, sqlx::Error> {
        let rule = sqlx::query_as::<_, TenantDistributionRule>(
            r#"
            INSERT INTO tenant_distribution_rules (
                tenant_id, beneficiary_id, share_ratio,
                priority, effective_from, effective_until
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(req.tenant_id)
        .bind(req.beneficiary_id)
        .bind(&req.share_ratio)
        .bind(req.priority.unwrap_or(0))
        .bind(req.effective_from.unwrap_or_else(Utc::now))
        .bind(req.effective_until)
        .fetch_one(pool)
        .await?;

        Ok(rule)
    }

    /// 根据 ID 查找规则
    pub async fn find_by_id(
        pool: &sqlx::PgPool,
        id: Uuid,
    ) -> Result<Option<TenantDistributionRule>, sqlx::Error> {
        let rule = sqlx::query_as::<_, TenantDistributionRule>(
            "SELECT * FROM tenant_distribution_rules WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(rule)
    }

    /// 查找租户的所有有效规则
    pub async fn find_by_tenant(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
    ) -> Result<Vec<TenantDistributionRule>, sqlx::Error> {
        let rules = sqlx::query_as::<_, TenantDistributionRule>(
            r#"
            SELECT * FROM tenant_distribution_rules
            WHERE tenant_id = $1
              AND enabled = TRUE
              AND effective_from <= NOW()
              AND (effective_until IS NULL OR effective_until > NOW())
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;

        Ok(rules)
    }

    /// 查找租户的所有规则（包括已禁用）
    pub async fn find_all_by_tenant(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
    ) -> Result<Vec<TenantDistributionRule>, sqlx::Error> {
        let rules = sqlx::query_as::<_, TenantDistributionRule>(
            "SELECT * FROM tenant_distribution_rules WHERE tenant_id = $1 ORDER BY priority DESC",
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;

        Ok(rules)
    }

    /// 更新规则
    pub async fn update(
        &self,
        pool: &sqlx::PgPool,
        req: &UpdateDistributionRuleRequest,
    ) -> Result<TenantDistributionRule, sqlx::Error> {
        let rule = sqlx::query_as::<_, TenantDistributionRule>(
            r#"
            UPDATE tenant_distribution_rules
            SET share_ratio = COALESCE($1, share_ratio),
                priority = COALESCE($2, priority),
                enabled = COALESCE($3, enabled),
                effective_until = COALESCE($4, effective_until),
                updated_at = NOW()
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(&req.share_ratio)
        .bind(req.priority)
        .bind(req.enabled)
        .bind(req.effective_until)
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(rule)
    }

    /// 删除规则
    pub async fn delete(&self, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tenant_distribution_rules WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 检查规则是否有效
    pub fn is_effective(&self) -> bool {
        if !self.enabled {
            return false;
        }

        let now = Utc::now();

        if self.effective_from > now {
            return false;
        }

        if let Some(effective_until) = self.effective_until
            && effective_until <= now
        {
            return false;
        }

        true
    }
}
