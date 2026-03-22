//! 冷却管理器
//!
//! 管理账号和 Provider 的冷却状态，支持基于错误率、RPM 等的动态冷却。

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 冷却条目
#[derive(Debug, Clone)]
pub struct CooldownEntry {
    /// 冷却开始时间
    pub started_at: Instant,
    /// 冷却持续时间
    pub duration: Duration,
    /// 冷却原因
    pub reason: CooldownReason,
}

/// 冷却原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CooldownReason {
    /// 连续错误
    ConsecutiveErrors,
    /// RPM 超限
    RpmLimitExceeded,
    /// 手动触发
    Manual,
    /// 熔断器触发
    CircuitBreaker,
    /// 其他原因
    Other(String),
}

impl CooldownEntry {
    /// 创建新的冷却条目
    pub fn new(duration: Duration, reason: CooldownReason) -> Self {
        Self {
            started_at: Instant::now(),
            duration,
            reason,
        }
    }

    /// 检查是否仍在冷却中
    pub fn is_active(&self) -> bool {
        Instant::now() < self.started_at + self.duration
    }

    /// 获取剩余冷却时间
    pub fn remaining(&self) -> Duration {
        let elapsed = Instant::now() - self.started_at;
        if elapsed < self.duration {
            self.duration - elapsed
        } else {
            Duration::from_secs(0)
        }
    }
}

/// 冷却管理器
#[derive(Debug)]
pub struct CooldownManager {
    /// 账号冷却状态
    account_cooldowns: DashMap<Uuid, CooldownEntry>,
    /// Provider 冷却状态
    provider_cooldowns: DashMap<String, CooldownEntry>,
    /// 默认冷却持续时间
    default_duration: Duration,
    /// 冷却计数（用于监控）
    cooldown_count: AtomicU64,
}

impl Default for CooldownManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CooldownManager {
    /// 创建新的冷却管理器
    pub fn new() -> Self {
        Self {
            account_cooldowns: DashMap::new(),
            provider_cooldowns: DashMap::new(),
            default_duration: Duration::from_secs(60),
            cooldown_count: AtomicU64::new(0),
        }
    }

    /// 创建带自定义默认冷却时间的管理器
    pub fn with_default_duration(default_duration: Duration) -> Self {
        Self {
            account_cooldowns: DashMap::new(),
            provider_cooldowns: DashMap::new(),
            default_duration,
            cooldown_count: AtomicU64::new(0),
        }
    }

    /// 设置账号冷却
    pub fn set_account_cooldown(
        &self,
        account_id: Uuid,
        duration: Option<Duration>,
        reason: CooldownReason,
    ) {
        let duration = duration.unwrap_or(self.default_duration);
        let entry = CooldownEntry::new(duration, reason);

        self.account_cooldowns.insert(account_id, entry);
        self.cooldown_count.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            account_id = %account_id,
            duration_secs = duration.as_secs(),
            "Account cooldown set"
        );
    }

    /// 设置 Provider 冷却
    pub fn set_provider_cooldown(
        &self,
        provider: impl Into<String>,
        duration: Option<Duration>,
        reason: CooldownReason,
    ) {
        let provider = provider.into();
        let duration = duration.unwrap_or(self.default_duration);
        let entry = CooldownEntry::new(duration, reason);

        self.provider_cooldowns.insert(provider.clone(), entry);
        self.cooldown_count.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            provider = %provider,
            duration_secs = duration.as_secs(),
            "Provider cooldown set"
        );
    }

    /// 检查账号是否在冷却中
    pub fn is_account_cooling(&self, account_id: &Uuid) -> bool {
        self.account_cooldowns
            .get(account_id)
            .map(|e| e.is_active())
            .unwrap_or(false)
    }

    /// 检查 Provider 是否在冷却中
    pub fn is_provider_cooling(&self, provider: &str) -> bool {
        self.provider_cooldowns
            .get(provider)
            .map(|e| e.is_active())
            .unwrap_or(false)
    }

    /// 获取账号冷却剩余时间
    pub fn account_cooldown_remaining(&self, account_id: &Uuid) -> Option<Duration> {
        self.account_cooldowns
            .get(account_id)
            .map(|e| e.remaining())
    }

    /// 获取 Provider 冷却剩余时间
    pub fn provider_cooldown_remaining(&self, provider: &str) -> Option<Duration> {
        self.provider_cooldowns
            .get(provider)
            .map(|e| e.remaining())
    }

    /// 清除账号冷却
    pub fn clear_account_cooldown(&self, account_id: &Uuid) {
        self.account_cooldowns.remove(account_id);
        tracing::debug!(account_id = %account_id, "Account cooldown cleared");
    }

    /// 清除 Provider 冷却
    pub fn clear_provider_cooldown(&self, provider: &str) {
        self.provider_cooldowns.remove(provider);
        tracing::debug!(provider = %provider, "Provider cooldown cleared");
    }

    /// 获取所有在冷却中的账号
    pub fn cooling_accounts(&self) -> Vec<(Uuid, CooldownEntry)> {
        self.account_cooldowns
            .iter()
            .filter(|entry| entry.value().is_active())
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// 获取所有在冷却中的 Provider
    pub fn cooling_providers(&self) -> Vec<(String, CooldownEntry)> {
        self.provider_cooldowns
            .iter()
            .filter(|entry| entry.value().is_active())
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// 清理过期的冷却条目
    pub fn cleanup_expired(&self) {
        let before_accounts = self.account_cooldowns.len();
        self.account_cooldowns.retain(|_, entry| entry.is_active());
        let after_accounts = self.account_cooldowns.len();

        let before_providers = self.provider_cooldowns.len();
        self.provider_cooldowns.retain(|_, entry| entry.is_active());
        let after_providers = self.provider_cooldowns.len();

        let removed = (before_accounts - after_accounts) + (before_providers - after_providers);
        if removed > 0 {
            tracing::debug!(removed, "Expired cooldown entries cleaned up");
        }
    }

    /// 获取冷却计数
    pub fn cooldown_count(&self) -> u64 {
        self.cooldown_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cooldown_entry() {
        let entry = CooldownEntry::new(Duration::from_secs(60), CooldownReason::ConsecutiveErrors);
        assert!(entry.is_active());
        assert!(entry.remaining() > Duration::from_secs(0));
    }

    #[test]
    fn test_account_cooldown() {
        let manager = CooldownManager::new();
        let account_id = Uuid::new_v4();

        // 设置冷却
        manager.set_account_cooldown(
            account_id,
            Some(Duration::from_secs(60)),
            CooldownReason::ConsecutiveErrors,
        );

        assert!(manager.is_account_cooling(&account_id));
        assert!(manager.account_cooldown_remaining(&account_id).is_some());

        // 清除冷却
        manager.clear_account_cooldown(&account_id);
        assert!(!manager.is_account_cooling(&account_id));
    }

    #[test]
    fn test_provider_cooldown() {
        let manager = CooldownManager::new();

        // 设置冷却
        manager.set_provider_cooldown(
            "openai",
            Some(Duration::from_secs(60)),
            CooldownReason::RpmLimitExceeded,
        );

        assert!(manager.is_provider_cooling("openai"));

        // 清理过期条目（应该还在冷却中）
        manager.cleanup_expired();
        assert!(manager.is_provider_cooling("openai"));
    }

    #[test]
    fn test_cooling_accounts() {
        let manager = CooldownManager::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        manager.set_account_cooldown(id1, None, CooldownReason::Manual);
        manager.set_account_cooldown(id2, None, CooldownReason::Manual);

        let cooling = manager.cooling_accounts();
        assert_eq!(cooling.len(), 2);
    }
}
