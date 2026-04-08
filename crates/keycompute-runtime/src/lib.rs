//! Runtime Core Layer
//!
//! 运行时核心层，提供加密和存储抽象。
//! 注意：Provider 健康状态和账号状态已移至 routing 模块。

pub mod crypto;
pub mod store;

#[cfg(feature = "redis")]
pub mod redis_store;

pub use crypto::{
    ApiKeyCrypto, CryptoError, EncryptedApiKey, decrypt_api_key, encrypt_api_key, global_crypto,
    set_global_crypto,
};
pub use store::{CleanupGuard, MemoryStore, RuntimeStore, StoreError, StoreResult};

#[cfg(feature = "redis")]
pub use redis_store::{RedisPoolConfig, RedisRuntimeStore, RedisStoreError};

use std::sync::Arc;

/// 运行时存储后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBackend {
    /// 内存后端
    Memory,
    /// Redis 后端
    Redis,
}

/// 运行时核心管理器
///
/// 提供加密和底层存储功能。
/// 注意：Provider 健康状态和账号状态已移至 routing 模块。
#[derive(Clone)]
pub struct RuntimeManager {
    /// 存储后端类型
    backend: RuntimeBackend,
    /// 存储实例
    store: Arc<dyn store::RuntimeStore>,
}

impl std::fmt::Debug for RuntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeManager")
            .field("backend", &self.backend)
            .field("store", &"<dyn RuntimeStore>")
            .finish()
    }
}

impl RuntimeManager {
    /// 创建新的运行时管理器（内存后端）
    pub fn new() -> Self {
        Self {
            backend: RuntimeBackend::Memory,
            store: Arc::new(store::MemoryStore::new()),
        }
    }

    /// 获取存储后端类型
    pub fn backend(&self) -> RuntimeBackend {
        self.backend
    }

    /// 获取存储实例
    pub fn store(&self) -> &Arc<dyn store::RuntimeStore> {
        &self.store
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "redis")]
impl RuntimeManager {
    /// 创建带 Redis 后端的运行时管理器
    ///
    /// # 参数
    /// - `redis_url`: Redis 连接 URL
    pub fn new_redis(redis_url: &str) -> Result<Self, redis_store::RedisStoreError> {
        let store = RedisRuntimeStore::new(redis_url)?;
        let store = Arc::new(store);

        Ok(Self {
            backend: RuntimeBackend::Redis,
            store,
        })
    }

    /// 创建带 Redis 后端的运行时管理器（带自定义前缀）
    pub fn new_redis_with_prefix(
        redis_url: &str,
        prefix: impl Into<String>,
    ) -> Result<Self, redis_store::RedisStoreError> {
        let store = RedisRuntimeStore::with_prefix(redis_url, prefix)?;
        let store = Arc::new(store);

        Ok(Self {
            backend: RuntimeBackend::Redis,
            store,
        })
    }

    /// 从配置创建 Redis 运行时管理器
    pub fn from_config(
        config: &redis_store::RedisPoolConfig,
    ) -> Result<Self, redis_store::RedisStoreError> {
        let store = RedisRuntimeStore::from_config(config)?;
        let store = Arc::new(store);

        Ok(Self {
            backend: RuntimeBackend::Redis,
            store,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_manager_new() {
        let manager = RuntimeManager::new();
        assert_eq!(manager.backend(), RuntimeBackend::Memory);
    }
}
