use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 用量日志模型（计费主账本）
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UsageLog {
    pub id: Uuid,
    pub request_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub produce_ai_key_id: Uuid,
    pub model_name: String,
    pub provider_name: String,
    pub account_id: Uuid,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    pub input_unit_price_snapshot: BigDecimal,
    pub output_unit_price_snapshot: BigDecimal,
    pub user_amount: BigDecimal,
    pub currency: String,
    pub usage_source: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// 创建用量日志请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUsageLogRequest {
    pub request_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub produce_ai_key_id: Uuid,
    pub model_name: String,
    pub provider_name: String,
    pub account_id: Uuid,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub input_unit_price_snapshot: BigDecimal,
    pub output_unit_price_snapshot: BigDecimal,
    pub user_amount: BigDecimal,
    pub currency: String,
    pub usage_source: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

/// 用量统计
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub total_amount: BigDecimal,
}

/// 用户用量统计（用于用户自服务 API）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UserUsageStats {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub total_cost: BigDecimal,
}

impl UsageLog {
    /// 创建用量日志
    pub async fn create(
        pool: &sqlx::PgPool,
        req: &CreateUsageLogRequest,
    ) -> Result<UsageLog, sqlx::Error> {
        let log = sqlx::query_as::<_, UsageLog>(
            r#"
            INSERT INTO usage_logs (
                request_id, tenant_id, user_id, produce_ai_key_id,
                model_name, provider_name, account_id,
                input_tokens, output_tokens, total_tokens,
                input_unit_price_snapshot, output_unit_price_snapshot,
                user_amount, currency, usage_source, status,
                started_at, finished_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $8 + $9,
                $10, $11, $12, $13, $14, $15, $16, $17
            )
            RETURNING *
            "#,
        )
        .bind(req.request_id)
        .bind(req.tenant_id)
        .bind(req.user_id)
        .bind(req.produce_ai_key_id)
        .bind(&req.model_name)
        .bind(&req.provider_name)
        .bind(req.account_id)
        .bind(req.input_tokens)
        .bind(req.output_tokens)
        .bind(&req.input_unit_price_snapshot)
        .bind(&req.output_unit_price_snapshot)
        .bind(&req.user_amount)
        .bind(&req.currency)
        .bind(&req.usage_source)
        .bind(&req.status)
        .bind(req.started_at)
        .bind(req.finished_at)
        .fetch_one(pool)
        .await?;

        Ok(log)
    }

    /// 根据 ID 查找用量日志
    pub async fn find_by_id(
        pool: &sqlx::PgPool,
        id: Uuid,
    ) -> Result<Option<UsageLog>, sqlx::Error> {
        let log = sqlx::query_as::<_, UsageLog>("SELECT * FROM usage_logs WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(log)
    }

    /// 根据请求 ID 查找用量日志
    pub async fn find_by_request_id(
        pool: &sqlx::PgPool,
        request_id: Uuid,
    ) -> Result<Option<UsageLog>, sqlx::Error> {
        let log = sqlx::query_as::<_, UsageLog>("SELECT * FROM usage_logs WHERE request_id = $1")
            .bind(request_id)
            .fetch_optional(pool)
            .await?;

        Ok(log)
    }

    /// 查找租户的用量日志
    pub async fn find_by_tenant(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UsageLog>, sqlx::Error> {
        let logs = sqlx::query_as::<_, UsageLog>(
            r#"
            SELECT * FROM usage_logs
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    /// 查找用户的用量日志
    pub async fn find_by_user(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UsageLog>, sqlx::Error> {
        let logs = sqlx::query_as::<_, UsageLog>(
            r#"
            SELECT * FROM usage_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    /// 获取租户用量统计
    pub async fn get_stats_by_tenant(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<UsageStats, sqlx::Error> {
        let stats = sqlx::query_as::<_, UsageStats>(
            r#"
            SELECT
                COUNT(*) as total_requests,
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                COALESCE(SUM(user_amount), 0) as total_amount
            FROM usage_logs
            WHERE tenant_id = $1
              AND created_at >= $2
              AND created_at < $3
            "#,
        )
        .bind(tenant_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await?;

        Ok(stats)
    }

    /// 获取用户用量统计
    pub async fn get_stats_by_user(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<UsageStats, sqlx::Error> {
        let stats = sqlx::query_as::<_, UsageStats>(
            r#"
            SELECT
                COUNT(*) as total_requests,
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                COALESCE(SUM(user_amount), 0) as total_amount
            FROM usage_logs
            WHERE user_id = $1
              AND created_at >= $2
              AND created_at < $3
            "#,
        )
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await?;

        Ok(stats)
    }

    /// 获取用户全部用量统计（不限时间范围）
    pub async fn get_user_stats(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<UserUsageStats, sqlx::Error> {
        let stats = sqlx::query_as::<_, UserUsageStats>(
            r#"
            SELECT
                COUNT(*) as total_requests,
                COALESCE(SUM(input_tokens), 0)::bigint as total_input_tokens,
                COALESCE(SUM(output_tokens), 0)::bigint as total_output_tokens,
                COALESCE(SUM(total_tokens), 0)::bigint as total_tokens,
                COALESCE(SUM(user_amount), 0) as total_cost
            FROM usage_logs
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(stats)
    }
}
