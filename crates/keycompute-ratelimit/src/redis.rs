//! Redis 限流器实现
//!
//! 基于 Redis 的分布式限流后端，支持多实例共享限流状态。

use crate::{RateLimitConfig, RateLimitKey, RateLimiter};
use async_trait::async_trait;
use keycompute_types::{KeyComputeError, Result};
use redis::{AsyncCommands, Client};
use std::sync::Arc;
use std::time::Duration;

/// Redis 限流器
///
/// 使用 Redis 实现分布式限流，支持：
/// - 滑动窗口限流
/// - 多实例共享限流状态
/// - 自动过期清理
#[derive(Debug, Clone)]
pub struct RedisRateLimiter {
    config: RateLimitConfig,
    client: Arc<Client>,
    window_size: Duration,
    key_prefix: String,
}

impl RedisRateLimiter {
    /// 创建新的 Redis 限流器
    ///
    /// # 参数
    /// - `redis_url`: Redis 连接 URL，如 "redis://127.0.0.1:6379"
    /// - `config`: 限流配置
    pub fn new(redis_url: &str, config: RateLimitConfig) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| KeyComputeError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            config,
            client: Arc::new(client),
            window_size: Duration::from_secs(60),
            key_prefix: "ratelimit".to_string(),
        })
    }

    /// 创建带自定义前缀的限流器
    pub fn with_prefix(
        redis_url: &str,
        config: RateLimitConfig,
        prefix: impl Into<String>,
    ) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| KeyComputeError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            config,
            client: Arc::new(client),
            window_size: Duration::from_secs(60),
            key_prefix: prefix.into(),
        })
    }

    /// 构建 Redis Key
    fn build_key(&self, key: &RateLimitKey) -> String {
        format!(
            "{}:{}:{}:{}:rpm",
            self.key_prefix, key.tenant_id, key.user_id, key.api_key_id
        )
    }

    /// 构建 Token 限流 Key
    fn build_token_key(&self, key: &RateLimitKey) -> String {
        format!(
            "{}:{}:{}:{}:tpm",
            self.key_prefix, key.tenant_id, key.user_id, key.api_key_id
        )
    }

    /// 获取 Redis 连接
    async fn get_conn(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis connection error: {}", e)))
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, key: &RateLimitKey) -> Result<bool> {
        let mut conn = self.get_conn().await?;
        let redis_key = self.build_key(key);

        // 使用 Redis 的滑动窗口限流
        // 1. 获取当前窗口内的请求数
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let window_start = now - self.window_size.as_secs() as i64;

        // 使用 ZREMRANGEBYSCORE 清理过期数据，然后 ZCARD 获取计数
        let _: () = conn
            .zrembyscore(&redis_key, 0, window_start)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        let count: u64 = conn
            .zcard(&redis_key)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        Ok(count < self.config.rpm_limit as u64)
    }

    async fn record(&self, key: &RateLimitKey) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let redis_key = self.build_key(key);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 添加当前请求时间戳到有序集合
        let _: () = conn
            .zadd(&redis_key, now, now)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        // 设置过期时间（窗口大小的2倍，确保数据自动清理）
        let expire_secs = self.window_size.as_secs() * 2;
        let _: () = conn
            .expire(&redis_key, expire_secs as i64)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}

impl RedisRateLimiter {
    /// 记录 Token 使用量（用于 TPM 限流）
    pub async fn record_tokens(&self, key: &RateLimitKey, tokens: u32) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let redis_key = self.build_token_key(key);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 添加 Token 使用量
        let _: () = conn
            .zadd(&redis_key, now, tokens as i64)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        // 设置过期时间
        let expire_secs = self.window_size.as_secs() * 2;
        let _: () = conn
            .expire(&redis_key, expire_secs as i64)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// 检查 Token 限流
    pub async fn check_tokens(&self, key: &RateLimitKey) -> Result<bool> {
        let mut conn = self.get_conn().await?;
        let redis_key = self.build_token_key(key);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let window_start = now - self.window_size.as_secs() as i64;

        // 清理过期数据
        let _: () = conn
            .zrembyscore(&redis_key, 0, window_start)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        // 获取窗口内的总 Token 数
        let tokens: Vec<i64> = conn
            .zrange(&redis_key, 0, -1)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        let total_tokens: i64 = tokens.iter().sum();

        Ok(total_tokens < self.config.tpm_limit as i64)
    }

    /// 清理所有限流数据（用于测试或重置）
    pub async fn flush_all(&self) -> Result<()> {
        let mut conn = self.get_conn().await?;

        // 使用 SCAN 查找并删除所有限流相关的 key
        let pattern = format!("{}:*", self.key_prefix);
        let mut iter: redis::AsyncIter<String> = conn
            .scan_match(&pattern)
            .await
            .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;

        while let Some(key) = iter.next_item().await {
            let _: () = conn
                .del(&key)
                .await
                .map_err(|e| KeyComputeError::Internal(format!("Redis error: {}", e)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_limiter() -> Option<RedisRateLimiter> {
        // 尝试连接本地 Redis，如果失败则跳过测试
        match RedisRateLimiter::new("redis://127.0.0.1:6379", RateLimitConfig::default()) {
            Ok(limiter) => Some(limiter),
            Err(_) => {
                eprintln!("Warning: Redis not available, skipping Redis tests");
                None
            }
        }
    }

    #[tokio::test]
    async fn test_redis_rate_limiter_check_and_record() {
        let Some(limiter) = create_test_limiter() else {
            return;
        };

        // 清理测试数据
        let _ = limiter.flush_all().await;

        let key = RateLimitKey::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

        // 第一次检查应该通过
        assert!(limiter.check(&key).await.unwrap());

        // 记录请求
        limiter.record(&key).await.unwrap();

        // 再次检查应该仍然通过（未达到限制）
        assert!(limiter.check(&key).await.unwrap());
    }

    #[tokio::test]
    async fn test_redis_rate_limiter_token_limit() {
        let Some(limiter) = create_test_limiter() else {
            return;
        };

        // 清理测试数据
        let _ = limiter.flush_all().await;

        let key = RateLimitKey::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

        // 检查 Token 限流
        assert!(limiter.check_tokens(&key).await.unwrap());

        // 记录一些 Token
        limiter.record_tokens(&key, 100).await.unwrap();

        // 仍然应该通过
        assert!(limiter.check_tokens(&key).await.unwrap());
    }
}
