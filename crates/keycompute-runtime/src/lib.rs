//! Runtime Core Layer
//!
//! 运行时状态写入中心，由 Gateway 驱动，供 Routing 只读。
//! 架构约束：是 Routing 读取的状态来源，由 LLM Gateway 驱动写入。

pub mod account_state;
pub mod cooldown;
pub mod crypto;
pub mod provider_health;
pub mod store;

#[cfg(feature = "redis")]
pub mod redis_store;

pub use account_state::{AccountState, AccountStateStore};
pub use cooldown::{CooldownEntry, CooldownManager, CooldownReason};
pub use crypto::{
    ApiKeyCrypto, CryptoError, EncryptedApiKey, decrypt_api_key, encrypt_api_key, global_crypto,
    set_global_crypto,
};
pub use provider_health::{ProviderHealth, ProviderHealthStore};
pub use store::RuntimeStore;

#[cfg(feature = "redis")]
pub use redis_store::RedisRuntimeStore;

use std::sync::Arc;

/// 运行时存储后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBackend {
    /// 内存后端
    Memory,
    /// Redis 后端
    Redis,
}

/// 运行时状态管理器
///
/// 集中管理所有运行时状态，是 Gateway 写入和 Routing 读取的统一入口
#[derive(Debug, Clone)]
pub struct RuntimeManager {
    /// 账号状态存储
    pub accounts: Arc<AccountStateStore>,
    /// Provider 健康状态存储
    pub providers: Arc<ProviderHealthStore>,
    /// 冷却管理器
    pub cooldown: Arc<CooldownManager>,
    /// 存储后端类型
    backend: RuntimeBackend,
}

impl RuntimeManager {
    /// 创建新的运行时管理器（内存后端）
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(AccountStateStore::new()),
            providers: Arc::new(ProviderHealthStore::new()),
            cooldown: Arc::new(CooldownManager::new()),
            backend: RuntimeBackend::Memory,
        }
    }

    /// 创建带自定义配置的运行时管理器
    pub fn with_stores(
        accounts: Arc<AccountStateStore>,
        providers: Arc<ProviderHealthStore>,
        cooldown: Arc<CooldownManager>,
    ) -> Self {
        Self {
            accounts,
            providers,
            cooldown,
            backend: RuntimeBackend::Memory,
        }
    }

    /// 获取存储后端类型
    pub fn backend(&self) -> RuntimeBackend {
        self.backend
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
    pub fn new_redis(redis_url: &str) -> Result<Self, redis::RedisError> {
        let _store = RedisRuntimeStore::new(redis_url)?;
        let _store = Arc::new(_store);

        // TODO: 后续可以扩展 AccountStateStore、ProviderHealthStore、CooldownManager
        // 支持 Redis 后端，目前保持内存实现
        Ok(Self {
            accounts: Arc::new(AccountStateStore::new()),
            providers: Arc::new(ProviderHealthStore::new()),
            cooldown: Arc::new(CooldownManager::new()),
            backend: RuntimeBackend::Redis,
        })
    }

    /// 创建带 Redis 后端的运行时管理器（带自定义前缀）
    pub fn new_redis_with_prefix(
        redis_url: &str,
        prefix: impl Into<String>,
    ) -> Result<Self, redis::RedisError> {
        let store = RedisRuntimeStore::with_prefix(redis_url, prefix)?;
        let _store = Arc::new(store);

        Ok(Self {
            accounts: Arc::new(AccountStateStore::new()),
            providers: Arc::new(ProviderHealthStore::new()),
            cooldown: Arc::new(CooldownManager::new()),
            backend: RuntimeBackend::Redis,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_manager_new() {
        let manager = RuntimeManager::new();
        assert!(Arc::strong_count(&manager.accounts) >= 1);
        assert!(Arc::strong_count(&manager.providers) >= 1);
        assert!(Arc::strong_count(&manager.cooldown) >= 1);
    }
}
