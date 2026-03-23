//! Redis 运行时状态存储实现
//!
//! 提供基于 Redis 的运行时状态存储后端，支持：
//! - 分布式状态共享
//! - 自动过期清理
//! - 高可用性

use crate::store::RuntimeStore;
use redis::{AsyncCommands, Client};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Redis 运行时存储
#[derive(Debug, Clone)]
pub struct RedisRuntimeStore {
    client: Arc<Client>,
    key_prefix: String,
    default_ttl: Duration,
}

impl RedisRuntimeStore {
    /// 创建新的 Redis 运行时存储
    ///
    /// # 参数
    /// - `redis_url`: Redis 连接 URL
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;

        Ok(Self {
            client: Arc::new(client),
            key_prefix: "keycompute:runtime".to_string(),
            default_ttl: Duration::from_secs(300),
        })
    }

    /// 创建带自定义前缀的存储
    pub fn with_prefix(
        redis_url: &str,
        prefix: impl Into<String>,
    ) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;

        Ok(Self {
            client: Arc::new(client),
            key_prefix: prefix.into(),
            default_ttl: Duration::from_secs(300),
        })
    }

    /// 设置默认 TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// 构建完整的 Redis Key
    fn build_key(&self, key: &str) -> String {
        format!("{}:{}", self.key_prefix, key)
    }

    /// 获取 Redis 连接
    async fn get_conn(&self) -> Result<redis::aio::MultiplexedConnection, redis::RedisError> {
        self.client.get_multiplexed_tokio_connection().await
    }
}

impl RuntimeStore for RedisRuntimeStore {
    fn get(&self, key: &str) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>> {
        let key = self.build_key(key);
        let client = Arc::clone(&self.client);

        Box::pin(async move {
            let mut conn = match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => conn,
                Err(_) => return None,
            };

            conn.get(&key).await.ok()
        })
    }

    fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let key = self.build_key(key);
        let value = value.to_string();
        let ttl = ttl.unwrap_or(self.default_ttl);
        let client = Arc::clone(&self.client);

        Box::pin(async move {
            if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                let _: Result<(), _> = conn.set_ex(&key, value, ttl.as_secs() as i64).await;
            }
        })
    }

    fn del(&self, key: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let key = self.build_key(key);
        let client = Arc::clone(&self.client);

        Box::pin(async move {
            if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                let _: Result<(), _> = conn.del(&key).await;
            }
        })
    }

    fn incr(&self, key: &str) -> Pin<Box<dyn Future<Output = i64> + Send + '_>> {
        let key = self.build_key(key);
        let client = Arc::clone(&self.client);

        Box::pin(async move {
            match client.get_multiplexed_tokio_connection().await {
                Ok(mut conn) => conn.incr(&key, 1i64).await.unwrap_or(1),
                Err(_) => 1,
            }
        })
    }

    fn decr(&self, key: &str) -> Pin<Box<dyn Future<Output = i64> + Send + '_>> {
        let key = self.build_key(key);
        let client = Arc::clone(&self.client);

        Box::pin(async move {
            match client.get_multiplexed_tokio_connection().await {
                Ok(mut conn) => conn.decr(&key, 1i64).await.unwrap_or(-1),
                Err(_) => -1,
            }
        })
    }

    fn expire(&self, key: &str, ttl: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let key = self.build_key(key);
        let client = Arc::clone(&self.client);

        Box::pin(async move {
            if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                let _: Result<(), _> = conn.expire(&key, ttl.as_secs() as i64).await;
            }
        })
    }
}

impl RedisRuntimeStore {
    /// 批量获取值
    pub async fn mget(&self, keys: &[&str]) -> Vec<Option<String>> {
        let full_keys: Vec<String> = keys.iter().map(|k| self.build_key(k)).collect();

        match self.get_conn().await {
            Ok(mut conn) => {
                let results: Vec<Option<String>> =
                    conn.mget(&full_keys).await.unwrap_or_else(|_| vec![]);
                results
            }
            Err(_) => vec![None; keys.len()],
        }
    }

    /// 批量设置值
    pub async fn mset(&self, kvs: &[(&str, &str)], ttl: Option<Duration>) {
        let ttl = ttl.unwrap_or(self.default_ttl);

        if let Ok(mut conn) = self.get_conn().await {
            for (key, value) in kvs {
                let full_key = self.build_key(key);
                let _: Result<(), _> = conn.set_ex(&full_key, *value, ttl.as_secs() as i64).await;
            }
        }
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> bool {
        match self.get_conn().await {
            Ok(mut conn) => conn.exists(self.build_key(key)).await.unwrap_or(false),
            Err(_) => false,
        }
    }

    /// 获取剩余过期时间（秒）
    pub async fn ttl(&self, key: &str) -> i64 {
        match self.get_conn().await {
            Ok(mut conn) => conn.ttl(self.build_key(key)).await.unwrap_or(-2),
            Err(_) => -2,
        }
    }

    /// 清理所有以当前前缀开头的键
    pub async fn flush_prefix(&self) -> Result<(), redis::RedisError> {
        let mut conn = self.get_conn().await?;
        let pattern = format!("{}:*", self.key_prefix);

        let mut iter: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;

        while let Some(key) = iter.next_item().await {
            let _: () = conn.del(&key).await?;
        }

        Ok(())
    }

    /// 获取 Redis 客户端
    pub fn client(&self) -> &Arc<Client> {
        &self.client
    }

    /// 获取 Key 前缀
    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }
}

/// Redis 连接池配置
#[derive(Debug, Clone)]
pub struct RedisPoolConfig {
    /// Redis URL
    pub url: String,
    /// 连接池大小
    pub pool_size: usize,
    /// 连接超时
    pub connect_timeout: Duration,
    /// 默认 TTL
    pub default_ttl: Duration,
    /// Key 前缀
    pub key_prefix: String,
}

impl Default for RedisPoolConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            connect_timeout: Duration::from_secs(5),
            default_ttl: Duration::from_secs(300),
            key_prefix: "keycompute:runtime".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> Option<RedisRuntimeStore> {
        match RedisRuntimeStore::new("redis://127.0.0.1:6379") {
            Ok(store) => Some(store),
            Err(_) => {
                eprintln!("Warning: Redis not available, skipping Redis tests");
                None
            }
        }
    }

    #[tokio::test]
    async fn test_redis_store_basic() {
        let Some(store) = create_test_store() else {
            return;
        };

        // 清理测试数据
        let _ = store.flush_prefix().await;

        // 测试 set/get
        store.set("test_key", "test_value", None).await;
        let value = store.get("test_key").await;
        assert_eq!(value, Some("test_value".to_string()));

        // 测试 del
        store.del("test_key").await;
        let value = store.get("test_key").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_redis_store_incr_decr() {
        let Some(store) = create_test_store() else {
            return;
        };

        let _ = store.flush_prefix().await;

        // 测试 incr
        let count1 = store.incr("counter").await;
        assert_eq!(count1, 1);

        let count2 = store.incr("counter").await;
        assert_eq!(count2, 2);

        // 测试 decr
        let count3 = store.decr("counter").await;
        assert_eq!(count3, 1);
    }

    #[tokio::test]
    async fn test_redis_store_ttl() {
        let Some(store) = create_test_store() else {
            return;
        };

        let _ = store.flush_prefix().await;

        // 设置带 TTL 的值
        store
            .set("ttl_key", "ttl_value", Some(Duration::from_secs(10)))
            .await;

        // 检查存在
        assert!(store.exists("ttl_key").await);

        // 检查 TTL
        let ttl = store.ttl("ttl_key").await;
        assert!(ttl > 0 && ttl <= 10);
    }
}
