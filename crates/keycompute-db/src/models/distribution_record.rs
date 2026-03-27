use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 分销记录模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DistributionRecord {
    pub id: Uuid,
    pub usage_log_id: Uuid,
    pub tenant_id: Uuid,
    pub beneficiary_id: Uuid,
    pub share_amount: BigDecimal,
    pub share_ratio: BigDecimal,
    /// 分销层级: level1, level2
    pub level: String,
    pub status: String,
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 创建分销记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDistributionRecordRequest {
    pub usage_log_id: Uuid,
    pub tenant_id: Uuid,
    pub beneficiary_id: Uuid,
    pub share_amount: BigDecimal,
    pub share_ratio: BigDecimal,
    /// 分销层级: level1, level2
    pub level: String,
}

/// 分销统计
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DistributionStats {
    pub total_records: i64,
    pub total_amount: BigDecimal,
    pub settled_amount: BigDecimal,
    pub pending_amount: BigDecimal,
}

/// 分销层级统计
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DistributionLevelStats {
    /// 一级分销收益
    pub level1_amount: BigDecimal,
    /// 二级分销收益
    pub level2_amount: BigDecimal,
    /// 一级分销记录数
    pub level1_count: i64,
    /// 二级分销记录数
    pub level2_count: i64,
}

impl DistributionRecord {
    /// 创建分销记录
    pub async fn create(
        pool: &sqlx::PgPool,
        req: &CreateDistributionRecordRequest,
    ) -> Result<DistributionRecord, sqlx::Error> {
        let record = sqlx::query_as::<_, DistributionRecord>(
            r#"
            INSERT INTO distribution_records (
                usage_log_id, tenant_id, beneficiary_id,
                share_amount, share_ratio, level, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending')
            RETURNING *
            "#,
        )
        .bind(req.usage_log_id)
        .bind(req.tenant_id)
        .bind(req.beneficiary_id)
        .bind(&req.share_amount)
        .bind(&req.share_ratio)
        .bind(&req.level)
        .fetch_one(pool)
        .await?;

        Ok(record)
    }

    /// 批量创建分销记录
    pub async fn create_many(
        pool: &sqlx::PgPool,
        requests: &[CreateDistributionRecordRequest],
    ) -> Result<Vec<DistributionRecord>, sqlx::Error> {
        let mut records = Vec::with_capacity(requests.len());

        for req in requests {
            let record = Self::create(pool, req).await?;
            records.push(record);
        }

        Ok(records)
    }

    /// 根据 ID 查找分销记录
    pub async fn find_by_id(
        pool: &sqlx::PgPool,
        id: Uuid,
    ) -> Result<Option<DistributionRecord>, sqlx::Error> {
        let record = sqlx::query_as::<_, DistributionRecord>(
            "SELECT * FROM distribution_records WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }

    /// 查找用量日志的所有分销记录
    pub async fn find_by_usage_log(
        pool: &sqlx::PgPool,
        usage_log_id: Uuid,
    ) -> Result<Vec<DistributionRecord>, sqlx::Error> {
        let records = sqlx::query_as::<_, DistributionRecord>(
            "SELECT * FROM distribution_records WHERE usage_log_id = $1",
        )
        .bind(usage_log_id)
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// 查找租户的分销记录
    pub async fn find_by_tenant(
        pool: &sqlx::PgPool,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DistributionRecord>, sqlx::Error> {
        let records = sqlx::query_as::<_, DistributionRecord>(
            r#"
            SELECT * FROM distribution_records
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

        Ok(records)
    }

    /// 查找受益人的分销记录
    pub async fn find_by_beneficiary(
        pool: &sqlx::PgPool,
        beneficiary_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DistributionRecord>, sqlx::Error> {
        let records = sqlx::query_as::<_, DistributionRecord>(
            r#"
            SELECT * FROM distribution_records
            WHERE beneficiary_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(beneficiary_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// 结算分销记录
    pub async fn settle(&self, pool: &sqlx::PgPool) -> Result<DistributionRecord, sqlx::Error> {
        let record = sqlx::query_as::<_, DistributionRecord>(
            r#"
            UPDATE distribution_records
            SET status = 'settled',
                settled_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(record)
    }

    /// 获取受益人统计
    pub async fn get_stats_by_beneficiary(
        pool: &sqlx::PgPool,
        beneficiary_id: Uuid,
    ) -> Result<DistributionStats, sqlx::Error> {
        let stats = sqlx::query_as::<_, DistributionStats>(
            r#"
            SELECT
                COUNT(*) as total_records,
                COALESCE(SUM(share_amount), 0) as total_amount,
                COALESCE(SUM(CASE WHEN status = 'settled' THEN share_amount ELSE 0 END), 0) as settled_amount,
                COALESCE(SUM(CASE WHEN status = 'pending' THEN share_amount ELSE 0 END), 0) as pending_amount
            FROM distribution_records
            WHERE beneficiary_id = $1
            "#,
        )
        .bind(beneficiary_id)
        .fetch_one(pool)
        .await?;

        Ok(stats)
    }

    /// 获取受益人按层级的统计
    pub async fn get_level_stats_by_beneficiary(
        pool: &sqlx::PgPool,
        beneficiary_id: Uuid,
    ) -> Result<DistributionLevelStats, sqlx::Error> {
        let stats = sqlx::query_as::<_, DistributionLevelStats>(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN level = 'level1' THEN share_amount ELSE 0 END), 0) as level1_amount,
                COALESCE(SUM(CASE WHEN level = 'level2' THEN share_amount ELSE 0 END), 0) as level2_amount,
                COUNT(CASE WHEN level = 'level1' THEN 1 END) as level1_count,
                COUNT(CASE WHEN level = 'level2' THEN 1 END) as level2_count
            FROM distribution_records
            WHERE beneficiary_id = $1
            "#,
        )
        .bind(beneficiary_id)
        .fetch_one(pool)
        .await?;

        Ok(stats)
    }

    /// 获取受益人在某个 usage_log 下的总收益（用于推荐人收益显示）
    pub async fn get_earnings_for_referral(
        pool: &sqlx::PgPool,
        beneficiary_id: Uuid,
        referred_user_id: Uuid,
    ) -> Result<BigDecimal, sqlx::Error> {
        // 查询该推荐用户产生的所有分销收益
        let result: Option<(BigDecimal,)> = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(dr.share_amount), 0)
            FROM distribution_records dr
            JOIN usage_logs ul ON dr.usage_log_id = ul.id
            WHERE dr.beneficiary_id = $1 AND ul.user_id = $2
            "#,
        )
        .bind(beneficiary_id)
        .bind(referred_user_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.0).unwrap_or(BigDecimal::from(0)))
    }
}
