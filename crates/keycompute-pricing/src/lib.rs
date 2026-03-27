//! Pricing Module
//!
//! 定价模块，只读，生成 PricingSnapshot。
//! 架构约束：不写任何状态，不参与路由或执行。

use keycompute_db::PricingModel;
use keycompute_types::{KeyComputeError, PricingSnapshot, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 定价服务
///
/// 负责从数据库加载模型价格，生成 PricingSnapshot
#[derive(Clone)]
pub struct PricingService {
    /// 数据库连接池（可选，用于测试时可以不提供）
    pool: Option<Arc<PgPool>>,
    /// 价格缓存
    cache: Arc<RwLock<HashMap<String, PricingSnapshot>>>,
}

impl std::fmt::Debug for PricingService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PricingService")
            .field("pool", &self.pool.as_ref().map(|_| "PgPool"))
            .field("cache", &"RwLock<HashMap>")
            .finish()
    }
}

impl Default for PricingService {
    fn default() -> Self {
        Self::new()
    }
}

impl PricingService {
    /// 创建新的定价服务（无数据库连接，使用默认价格）
    pub fn new() -> Self {
        Self {
            pool: None,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建带数据库连接的定价服务
    pub fn with_pool(pool: Arc<PgPool>) -> Self {
        Self {
            pool: Some(pool),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建价格快照（固化到 RequestContext）
    ///
    /// 从数据库或缓存加载指定模型的价格
    pub async fn create_snapshot(
        &self,
        model_name: &str,
        tenant_id: &Uuid,
    ) -> Result<PricingSnapshot> {
        // 先检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(snapshot) = cache.get(model_name) {
                tracing::debug!(model = %model_name, "Pricing snapshot from cache");
                return Ok(snapshot.clone());
            }
        }

        // 尝试从数据库加载
        let snapshot = if let Some(pool) = &self.pool {
            self.load_from_database(pool, model_name, tenant_id).await?
        } else {
            // 无数据库连接时使用默认价格
            self.get_default_pricing(model_name)
        };

        // 写入缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(model_name.to_string(), snapshot.clone());
        }

        tracing::debug!(model = %model_name, price = ?snapshot, "Created pricing snapshot");
        Ok(snapshot)
    }

    /// 从数据库加载价格
    async fn load_from_database(
        &self,
        pool: &PgPool,
        model_name: &str,
        tenant_id: &Uuid,
    ) -> Result<PricingSnapshot> {
        // 尝试按租户+模型名查找，支持任意 provider
        let pricing = PricingModel::find_by_model(pool, *tenant_id, model_name, "openai")
            .await
            .map_err(|e| {
                KeyComputeError::DatabaseError(format!("Failed to load pricing: {}", e))
            })?;

        if let Some(p) = pricing {
            return Ok(PricingSnapshot {
                model_name: p.model_name,
                currency: p.currency,
                input_price_per_1k: bigdecimal_to_decimal(&p.input_price_per_1k),
                output_price_per_1k: bigdecimal_to_decimal(&p.output_price_per_1k),
            });
        }

        // 尝试查找默认定价
        let defaults = PricingModel::find_defaults(pool).await.map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to load default pricing: {}", e))
        })?;

        for p in defaults {
            if p.model_name == model_name {
                return Ok(PricingSnapshot {
                    model_name: p.model_name,
                    currency: p.currency,
                    input_price_per_1k: bigdecimal_to_decimal(&p.input_price_per_1k),
                    output_price_per_1k: bigdecimal_to_decimal(&p.output_price_per_1k),
                });
            }
        }

        // 未找到，使用默认价格
        tracing::warn!(
            model = %model_name,
            tenant_id = %tenant_id,
            "No pricing found in database, using default"
        );
        Ok(self.get_default_pricing(model_name))
    }

    /// 获取默认定价
    fn get_default_pricing(&self, model_name: &str) -> PricingSnapshot {
        // 根据模型名称返回默认价格
        let (input_price, output_price) = match model_name {
            "gpt-4o" => (
                Decimal::from(500) / Decimal::from(1000),
                Decimal::from(1500) / Decimal::from(1000),
            ),
            "gpt-4o-mini" => (
                Decimal::from(150) / Decimal::from(1000),
                Decimal::from(600) / Decimal::from(1000),
            ),
            "gpt-4-turbo" => (
                Decimal::from(1000) / Decimal::from(1000),
                Decimal::from(3000) / Decimal::from(1000),
            ),
            "gpt-3.5-turbo" => (
                Decimal::from(50) / Decimal::from(1000),
                Decimal::from(150) / Decimal::from(1000),
            ),
            _ => (
                Decimal::from(100) / Decimal::from(1000),
                Decimal::from(300) / Decimal::from(1000),
            ),
        };

        PricingSnapshot {
            model_name: model_name.to_string(),
            currency: "CNY".to_string(),
            input_price_per_1k: input_price,
            output_price_per_1k: output_price,
        }
    }

    /// 清除缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::info!("Pricing cache cleared");
    }

    /// 预热缓存（从数据库加载所有默认定价）
    pub async fn warmup_cache(&self) -> Result<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        let defaults = PricingModel::find_defaults(pool).await.map_err(|e| {
            KeyComputeError::DatabaseError(format!("Failed to load default pricing: {}", e))
        })?;

        let mut cache = self.cache.write().await;
        for p in defaults {
            let snapshot = PricingSnapshot {
                model_name: p.model_name.clone(),
                currency: p.currency.clone(),
                input_price_per_1k: bigdecimal_to_decimal(&p.input_price_per_1k),
                output_price_per_1k: bigdecimal_to_decimal(&p.output_price_per_1k),
            };
            cache.insert(p.model_name, snapshot);
        }

        tracing::info!(count = cache.len(), "Pricing cache warmed up");
        Ok(())
    }

    /// 计算请求费用
    pub fn calculate_cost(
        &self,
        input_tokens: u32,
        output_tokens: u32,
        pricing: &PricingSnapshot,
    ) -> Decimal {
        let input_cost =
            Decimal::from(input_tokens) * pricing.input_price_per_1k / Decimal::from(1000);
        let output_cost =
            Decimal::from(output_tokens) * pricing.output_price_per_1k / Decimal::from(1000);
        input_cost + output_cost
    }

    /// 检查是否已配置数据库连接
    ///
    /// 用于启动时验证配置
    pub fn has_pool(&self) -> bool {
        self.pool.is_some()
    }
}

/// 将 BigDecimal 转换为 Decimal
fn bigdecimal_to_decimal(value: &bigdecimal::BigDecimal) -> Decimal {
    // BigDecimal -> String -> Decimal
    let s = value.to_string();
    s.parse().unwrap_or(Decimal::ZERO)
}
