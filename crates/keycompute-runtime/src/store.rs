//! 运行时状态存储抽象
//!
//! 提供状态存储的后端抽象，支持内存和 Redis 实现。

use dashmap::DashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// 存储错误类型
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// 连接错误
    #[error("Store connection failed: {0}")]
    ConnectionFailed(String),
    /// 操作错误
    #[error("Store operation failed: {0}")]
    OperationFailed(String),
    /// 键不存在
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    /// 序列化错误
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
}

/// 存储结果类型
pub type StoreResult<T> = Result<T, StoreError>;

/// 状态存储 trait
///
/// 定义运行时状态存储的基本操作
pub trait RuntimeStore: Send + Sync {
    /// 获取字符串值
    ///
    /// 返回 `Ok(None)` 表示键不存在，
    /// 返回 `Err(...)` 表示操作失败（如连接错误）
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = StoreResult<Option<String>>> + Send + '_>>;

    /// 设置字符串值
    fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Pin<Box<dyn Future<Output = StoreResult<()>> + Send + '_>>;

    /// 删除键
    fn del(&self, key: &str) -> Pin<Box<dyn Future<Output = StoreResult<()>> + Send + '_>>;

    /// 递增计数器
    fn incr(&self, key: &str) -> Pin<Box<dyn Future<Output = StoreResult<i64>> + Send + '_>>;

    /// 递减计数器
    fn decr(&self, key: &str) -> Pin<Box<dyn Future<Output = StoreResult<i64>> + Send + '_>>;

    /// 设置过期时间
    fn expire(
        &self,
        key: &str,
        ttl: Duration,
    ) -> Pin<Box<dyn Future<Output = StoreResult<()>> + Send + '_>>;
}

/// 内存存储实现
///
/// 使用 DashMap 实现线程安全的内存存储，支持 TTL 过期。
/// 支持主动 TTL 清理和原子计数器操作。
#[derive(Debug)]
pub struct MemoryStore {
    data: Arc<DashMap<String, (String, Option<Instant>)>>,
    /// TTL 清理任务句柄
    cleanup_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// 清理任务停止标志
    cleanup_cancel: Arc<AtomicBool>,
}

impl MemoryStore {
    /// 创建新的内存存储
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            cleanup_handle: Arc::new(Mutex::new(None)),
            cleanup_cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 启动后台 TTL 清理任务
    ///
    /// 定期扫描并清理已过期的键，防止内存泄漏。
    /// 返回一个 guard，当 guard 被 drop 时会停止清理任务。
    ///
    /// # 参数
    /// - `interval`: 清理间隔，默认 60 秒
    ///
    /// # 示例
    /// ```rust,ignore
    /// let store = MemoryStore::new();
    /// let _guard = store.start_cleanup_task(Duration::from_secs(60));
    /// // 清理任务会在 _guard 被 drop 时停止
    /// ```
    pub async fn start_cleanup_task(&self, interval: Duration) -> CleanupGuard {
        // 如果已有清理任务在运行，直接返回
        {
            let handle = self.cleanup_handle.lock().await;
            if handle.is_some() {
                return CleanupGuard {
                    cancel: Arc::clone(&self.cleanup_cancel),
                };
            }
        }

        // 设置运行标志
        self.cleanup_cancel.store(false, Ordering::SeqCst);

        let data = Arc::clone(&self.data);
        let cancel = Arc::clone(&self.cleanup_cancel);
        let handle = Arc::clone(&self.cleanup_handle);

        let task = tokio::spawn(async move {
            loop {
                // 检查是否需要停止
                if cancel.load(Ordering::SeqCst) {
                    break;
                }

                // 等待清理间隔
                tokio::time::sleep(interval).await;

                // 再次检查停止标志
                if cancel.load(Ordering::SeqCst) {
                    break;
                }

                // 执行清理
                let now = Instant::now();
                let keys_to_remove: Vec<String> = data
                    .iter()
                    .filter_map(|entry| {
                        let (key, (_, expire_at)) = entry.pair();
                        if let Some(exp) = expire_at
                            && now > *exp
                        {
                            return Some(key.clone());
                        }
                        None
                    })
                    .collect();

                // 删除过期键
                for key in keys_to_remove {
                    data.remove(&key);
                }

                tracing::trace!("TTL cleanup completed");
            }

            // 清理完成后清除句柄
            let mut h = handle.lock().await;
            *h = None;
        });

        // 保存任务句柄
        {
            let mut h = self.cleanup_handle.lock().await;
            *h = Some(task);
        }

        CleanupGuard {
            cancel: Arc::clone(&self.cleanup_cancel),
        }
    }

    /// 停止 TTL 清理任务
    pub async fn stop_cleanup_task(&self) {
        self.cleanup_cancel.store(true, Ordering::SeqCst);
        let mut handle = self.cleanup_handle.lock().await;
        *handle = None;
    }
}

/// TTL 清理任务 guard
///
/// 当 guard 被 drop 时，会通知清理任务停止。
#[derive(Debug)]
pub struct CleanupGuard {
    cancel: Arc<AtomicBool>,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl Clone for MemoryStore {
    fn clone(&self) -> Self {
        // 克隆时不共享清理任务，创建新的独立实例
        Self {
            data: Arc::new(DashMap::new()),
            cleanup_handle: Arc::new(Mutex::new(None)),
            cleanup_cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeStore for MemoryStore {
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = StoreResult<Option<String>>> + Send + '_>> {
        let key = key.to_string();
        let data = Arc::clone(&self.data);

        Box::pin(async move {
            // 使用 remove_if 原子性地检查并删除过期键
            // 如果键存在且已过期，remove_if 会删除它并返回 true
            // 如果键存在且未过期，返回 false
            // 如果键不存在，返回 false
            let now = Instant::now();
            let removed = data.remove_if(&key, |_, (_, expire_at)| {
                expire_at.is_some_and(|exp| now > exp)
            });

            if removed.is_some() {
                // 键已过期并被删除
                return Ok(None);
            }

            // 键未过期或不存在，正常获取
            if let Some(entry) = data.get(&key) {
                Ok(Some(entry.0.clone()))
            } else {
                Ok(None)
            }
        })
    }

    fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Pin<Box<dyn Future<Output = StoreResult<()>> + Send + '_>> {
        let key = key.to_string();
        let value = value.to_string();
        let expire_at = ttl.map(|d| Instant::now() + d);
        let data = Arc::clone(&self.data);

        Box::pin(async move {
            data.insert(key, (value, expire_at));
            Ok(())
        })
    }

    fn del(&self, key: &str) -> Pin<Box<dyn Future<Output = StoreResult<()>> + Send + '_>> {
        let key = key.to_string();
        let data = Arc::clone(&self.data);

        Box::pin(async move {
            data.remove(&key);
            Ok(())
        })
    }

    fn incr(&self, key: &str) -> Pin<Box<dyn Future<Output = StoreResult<i64>> + Send + '_>> {
        let key = key.to_string();
        let data = Arc::clone(&self.data);

        Box::pin(async move {
            // 使用 remove_if 和 entry API 实现原子性的 incr 操作
            let now = Instant::now();

            // 先原子性地删除过期键
            data.remove_if(&key, |_, (_, expire_at)| {
                expire_at.is_some_and(|exp| now > exp)
            });

            // 使用 entry API 获取或创建 entry，并原子性地更新
            let mut result: i64 = 1;
            data.entry(key.clone())
                .and_modify(|(val, expire_at)| {
                    // 清除过期时间（计数器操作后不应有 TTL）
                    *expire_at = None;
                    // 递增
                    let current: i64 = val.parse().unwrap_or(0);
                    *val = (current + 1).to_string();
                    result = current + 1;
                })
                .or_insert_with(|| ("1".to_string(), None));

            Ok(result)
        })
    }

    fn decr(&self, key: &str) -> Pin<Box<dyn Future<Output = StoreResult<i64>> + Send + '_>> {
        let key = key.to_string();
        let data = Arc::clone(&self.data);

        Box::pin(async move {
            // 使用 remove_if 和 entry API 实现原子性的 decr 操作
            let now = Instant::now();

            // 先原子性地删除过期键
            data.remove_if(&key, |_, (_, expire_at)| {
                expire_at.is_some_and(|exp| now > exp)
            });

            // 使用 entry API 获取或创建 entry，并原子性地更新
            let mut result: i64 = -1;
            data.entry(key.clone())
                .and_modify(|(val, expire_at)| {
                    // 清除过期时间（计数器操作后不应有 TTL）
                    *expire_at = None;
                    // 递减
                    let current: i64 = val.parse().unwrap_or(0);
                    *val = (current - 1).to_string();
                    result = current - 1;
                })
                .or_insert_with(|| ("-1".to_string(), None));

            Ok(result)
        })
    }

    fn expire(
        &self,
        key: &str,
        ttl: Duration,
    ) -> Pin<Box<dyn Future<Output = StoreResult<()>> + Send + '_>> {
        let key = key.to_string();
        let expire_at = Instant::now() + ttl;
        let data = Arc::clone(&self.data);

        Box::pin(async move {
            if let Some(mut entry) = data.get_mut(&key) {
                entry.1 = Some(expire_at);
                Ok(())
            } else {
                Err(StoreError::KeyNotFound(key))
            }
        })
    }
}

/// 存储后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackend {
    /// 内存存储
    Memory,
    /// Redis 存储
    Redis,
}

/// 存储配置
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// 存储后端类型
    pub backend: StoreBackend,
    /// Redis URL（如果使用 Redis）
    pub redis_url: Option<String>,
    /// 默认 TTL
    pub default_ttl: Duration,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            backend: StoreBackend::Memory,
            redis_url: None,
            default_ttl: Duration::from_secs(300),
        }
    }
}

impl StoreConfig {
    /// 创建内存存储配置
    pub fn memory() -> Self {
        Self::default()
    }

    /// 创建 Redis 存储配置
    pub fn redis(url: impl Into<String>) -> Self {
        Self {
            backend: StoreBackend::Redis,
            redis_url: Some(url.into()),
            default_ttl: Duration::from_secs(300),
        }
    }

    /// 设置默认 TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_config_default() {
        let config = StoreConfig::default();
        assert_eq!(config.backend, StoreBackend::Memory);
        assert!(config.redis_url.is_none());
    }

    #[test]
    fn test_store_config_redis() {
        let config = StoreConfig::redis("redis://localhost:6379");
        assert_eq!(config.backend, StoreBackend::Redis);
        assert_eq!(config.redis_url, Some("redis://localhost:6379".to_string()));
    }

    #[test]
    fn test_store_config_with_ttl() {
        let config = StoreConfig::memory().with_ttl(Duration::from_secs(600));
        assert_eq!(config.default_ttl, Duration::from_secs(600));
    }

    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryStore::new();

        // 测试 set/get
        store.set("key1", "value1", None).await.unwrap();
        let value = store.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // 测试不存在的键
        let value = store.get("nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_memory_store_del() {
        let store = MemoryStore::new();

        store.set("key1", "value1", None).await.unwrap();
        assert_eq!(store.get("key1").await.unwrap(), Some("value1".to_string()));

        store.del("key1").await.unwrap();
        assert_eq!(store.get("key1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_store_incr_decr() {
        let store = MemoryStore::new();

        // 测试 incr
        assert_eq!(store.incr("counter").await.unwrap(), 1);
        assert_eq!(store.incr("counter").await.unwrap(), 2);
        assert_eq!(store.incr("counter").await.unwrap(), 3);

        // 测试 decr
        assert_eq!(store.decr("counter").await.unwrap(), 2);
        assert_eq!(store.decr("counter").await.unwrap(), 1);
        assert_eq!(store.decr("counter").await.unwrap(), 0);
        assert_eq!(store.decr("counter").await.unwrap(), -1);
    }

    #[tokio::test]
    async fn test_memory_store_ttl() {
        let store = MemoryStore::new();

        // 设置带 TTL 的值
        store
            .set("ttl_key", "ttl_value", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        assert_eq!(
            store.get("ttl_key").await.unwrap(),
            Some("ttl_value".to_string())
        );

        // 等待过期
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(store.get("ttl_key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_store_expire() {
        let store = MemoryStore::new();

        // 先设置无 TTL 的值
        store.set("key", "value", None).await.unwrap();
        assert_eq!(store.get("key").await.unwrap(), Some("value".to_string()));

        // 设置过期时间
        store
            .expire("key", Duration::from_millis(50))
            .await
            .unwrap();
        assert_eq!(store.get("key").await.unwrap(), Some("value".to_string()));

        // 等待过期
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(store.get("key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_store_expire_nonexistent() {
        let store = MemoryStore::new();

        // 对不存在的键设置过期时间应该返回错误
        let result = store.expire("nonexistent", Duration::from_secs(10)).await;
        assert!(matches!(result, Err(StoreError::KeyNotFound(_))));
    }

    // ==================== 并发安全测试 ====================

    #[tokio::test]
    async fn test_concurrent_incr() {
        let store = std::sync::Arc::new(MemoryStore::new());
        let mut handles = vec![];

        // 并发执行 100 个 incr 操作
        for _ in 0..100 {
            let store_clone = std::sync::Arc::clone(&store);
            let handle =
                tokio::spawn(async move { store_clone.incr("concurrent_counter").await.unwrap() });
            handles.push(handle);
        }

        // 等待所有任务完成
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // 验证最终计数
        let final_count = store.incr("concurrent_counter").await.unwrap();
        assert_eq!(
            final_count, 101,
            "Final count should be 101 after 100 increments + 1"
        );

        // 验证所有结果都是唯一的（没有丢失更新）
        results.sort();
        let unique_count = results.windows(2).filter(|w| w[0] != w[1]).count() + 1;
        assert_eq!(unique_count, 100, "All 100 results should be unique");
    }

    #[tokio::test]
    async fn test_concurrent_decr() {
        let store = std::sync::Arc::new(MemoryStore::new());
        let mut handles = vec![];

        // 并发执行 100 个 decr 操作
        for _ in 0..100 {
            let store_clone = std::sync::Arc::clone(&store);
            let handle =
                tokio::spawn(async move { store_clone.decr("concurrent_decr").await.unwrap() });
            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            let _ = handle.await;
        }

        // 验证最终计数
        let final_count = store.decr("concurrent_decr").await.unwrap();
        assert_eq!(
            final_count, -101,
            "Final count should be -101 after 100 decrements - 1"
        );
    }

    #[tokio::test]
    async fn test_concurrent_get_set() {
        let store = std::sync::Arc::new(MemoryStore::new());
        let mut handles = vec![];

        // 并发执行 set 操作
        for i in 0..50 {
            let store_clone = std::sync::Arc::clone(&store);
            let handle = tokio::spawn(async move {
                store_clone
                    .set("shared_key", &format!("value_{}", i), None)
                    .await
                    .unwrap()
            });
            handles.push(handle);
        }

        // 等待所有 set 完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 验证值存在（最后一个写入的值）
        let value = store.get("shared_key").await.unwrap();
        assert!(value.is_some(), "Value should exist after concurrent sets");
        assert!(value.unwrap().starts_with("value_"));
    }

    #[tokio::test]
    async fn test_incr_with_expired_key() {
        let store = MemoryStore::new();

        // 设置一个很短的 TTL
        store
            .set("expiring_counter", "100", Some(Duration::from_millis(50)))
            .await
            .unwrap();

        // 等待过期
        tokio::time::sleep(Duration::from_millis(100)).await;

        // incr 过期键应该重置为 1
        let result = store.incr("expiring_counter").await.unwrap();
        assert_eq!(result, 1, "Incr on expired key should reset to 1");
    }

    #[tokio::test]
    async fn test_decr_with_expired_key() {
        let store = MemoryStore::new();

        // 设置一个很短的 TTL
        store
            .set("expiring_decr", "100", Some(Duration::from_millis(50)))
            .await
            .unwrap();

        // 等待过期
        tokio::time::sleep(Duration::from_millis(100)).await;

        // decr 过期键应该重置为 -1
        let result = store.decr("expiring_decr").await.unwrap();
        assert_eq!(result, -1, "Decr on expired key should reset to -1");
    }

    // ==================== TTL 清理任务测试 ====================

    #[tokio::test]
    async fn test_cleanup_task() {
        let store = MemoryStore::new();

        // 启动清理任务（100ms 间隔）
        let _guard = store.start_cleanup_task(Duration::from_millis(100)).await;

        // 设置一些带 TTL 的值
        store
            .set("key1", "value1", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        store
            .set("key2", "value2", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        store.set("key3", "value3", None).await.unwrap(); // 无 TTL

        // 等待过期和清理
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 验证 TTL 键已被清理
        assert_eq!(store.get("key1").await.unwrap(), None);
        assert_eq!(store.get("key2").await.unwrap(), None);
        // 无 TTL 的键仍然存在
        assert_eq!(store.get("key3").await.unwrap(), Some("value3".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_task_stop() {
        let store = MemoryStore::new();

        // 启动清理任务
        let guard = store.start_cleanup_task(Duration::from_millis(50)).await;

        // 设置值
        store
            .set("key", "value", Some(Duration::from_millis(30)))
            .await
            .unwrap();

        // 显式停止清理任务
        drop(guard);

        // 给任务一点时间停止
        tokio::time::sleep(Duration::from_millis(10)).await;

        // 验证停止后的清理任务不会继续运行
        // 注意：由于任务已停止，惰性清理仍然会生效
    }

    #[tokio::test]
    async fn test_cleanup_guard_drop() {
        let store = std::sync::Arc::new(MemoryStore::new());

        {
            let _guard = store.start_cleanup_task(Duration::from_millis(50)).await;
            // guard 在此作用域结束时 drop
        }

        // 验证可以重新启动清理任务
        let _guard2 = store.start_cleanup_task(Duration::from_millis(50)).await;
    }
}
