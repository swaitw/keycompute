use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 定价模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PricingModel {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub currency: String,
    pub input_price_per_1k: BigDecimal,
    pub output_price_per_1k: BigDecimal,
    pub is_default: bool,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建定价请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePricingRequest {
    pub tenant_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub currency: Option<String>,
    pub input_price_per_1k: BigDecimal,
    pub output_price_per_1k: BigDecimal,
    pub is_default: Option<bool>,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_until: Option<DateTime<Utc>>,
}

/// 更新定价请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePricingRequest {
    pub input_price_per_1k: Option<BigDecimal>,
    pub output_price_per_1k: Option<BigDecimal>,
    pub effective_until: Option<DateTime<Utc>>,
}

impl PricingModel {
    /// 创建新定价
    pub async fn create(
        pool: &sqlx::PgPool,
        req: &CreatePricingRequest,
    ) -> Result<PricingModel, sqlx::Error> {
        let pricing = sqlx::query_as::<_, PricingModel>(
            r#"
            INSERT INTO pricing_models (
                tenant_id, model_name, provider, currency,
                input_price_per_1k, output_price_per_1k,
                is_default, effective_from, effective_until
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(req.tenant_id)
        .bind(&req.model_name)
        .bind(&req.provider)
        .bind(req.currency.as_deref().unwrap_or("CNY"))
        .bind(&req.input_price_per_1k)
        .bind(&req.output_price_per_1k)
        .bind(req.is_default.unwrap_or(false))
        .bind(req.effective_from.unwrap_or_else(Utc::now))
        .bind(req.effective_until)
        .fetch_one(pool)
        .await?;

        Ok(pricing)
    }

    /// 根据 ID 查找定价
    pub async fn find_by_id(
        pool: &sqlx::PgPool,
        id: Uuid,
    ) -> Result<Option<PricingModel>, sqlx::Error> {
        let pricing =
            sqlx::query_as::<_, PricingModel>("SELECT * FROM pricing_models WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await?;

        Ok(pricing)
    }

    /// 查找租户的所有定价
    pub async fn find_by_tenant(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
    ) -> Result<Vec<PricingModel>, sqlx::Error> {
        let pricing = sqlx::query_as::<_, PricingModel>(
            r#"
            SELECT * FROM pricing_models
            WHERE tenant_id = $1
               OR (tenant_id IS NULL AND is_default = TRUE)
            ORDER BY model_name, tenant_id NULLS LAST
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;

        Ok(pricing)
    }

    /// 查找特定模型的定价（优先租户定价，其次默认定价）
    pub async fn find_by_model(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
        model_name: &str,
        provider: &str,
    ) -> Result<Option<PricingModel>, sqlx::Error> {
        let pricing = sqlx::query_as::<_, PricingModel>(
            r#"
            SELECT * FROM pricing_models
            WHERE model_name = $1
              AND provider = $2
              AND effective_from <= NOW()
              AND (effective_until IS NULL OR effective_until > NOW())
              AND (tenant_id = $3 OR (tenant_id IS NULL AND is_default = TRUE))
            ORDER BY tenant_id NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(model_name)
        .bind(provider)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;

        Ok(pricing)
    }

    /// 查找所有默认定价
    pub async fn find_defaults(pool: &sqlx::PgPool) -> Result<Vec<PricingModel>, sqlx::Error> {
        let pricing = sqlx::query_as::<_, PricingModel>(
            r#"
            SELECT * FROM pricing_models
            WHERE is_default = TRUE
              AND effective_from <= NOW()
              AND (effective_until IS NULL OR effective_until > NOW())
            ORDER BY model_name
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(pricing)
    }

    /// 更新定价
    pub async fn update(
        &self,
        pool: &sqlx::PgPool,
        req: &UpdatePricingRequest,
    ) -> Result<PricingModel, sqlx::Error> {
        let pricing = sqlx::query_as::<_, PricingModel>(
            r#"
            UPDATE pricing_models
            SET input_price_per_1k = COALESCE($1, input_price_per_1k),
                output_price_per_1k = COALESCE($2, output_price_per_1k),
                effective_until = COALESCE($3, effective_until),
                updated_at = NOW()
            WHERE id = $4
            RETURNING *
            "#,
        )
        .bind(&req.input_price_per_1k)
        .bind(&req.output_price_per_1k)
        .bind(req.effective_until)
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(pricing)
    }

    /// 删除定价
    pub async fn delete(&self, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM pricing_models WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 检查定价是否有效
    pub fn is_effective(&self) -> bool {
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
