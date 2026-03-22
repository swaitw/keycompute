//! Runtime Core Layer
//!
//! 运行时状态写入中心，由 Gateway 驱动，供 Routing 只读。
//! 架构约束：是 Routing 读取的状态来源，由 LLM Gateway 驱动写入。

pub mod account_state;
pub mod cooldown;
pub mod provider_health;
pub mod store;

pub use account_state::{AccountState, AccountStateStore};
pub use cooldown::CooldownManager;
pub use provider_health::{ProviderHealth, ProviderHealthStore};
pub use store::RuntimeStore;

use std::sync::Arc;

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
}

impl RuntimeManager {
    /// 创建新的运行时管理器
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(AccountStateStore::new()),
            providers: Arc::new(ProviderHealthStore::new()),
            cooldown: Arc::new(CooldownManager::new()),
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
        }
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
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
