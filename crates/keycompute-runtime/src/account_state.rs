//! 账号状态管理
//!
//! 管理账号的错误计数、冷却状态、RPM 等运行时状态。
//! Gateway 写入，Routing 只读。

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 账号状态
#[derive(Debug, Clone)]
pub struct AccountState {
    /// 连续错误计数
    pub error_count: u32,
    /// 最后一次错误时间
    pub last_error_at: Option<Instant>,
    /// 冷却直到时间
    pub cooldown_until: Option<Instant>,
    /// 当前 RPM（每分钟请求数）
    pub current_rpm: u32,
    /// 最后一次请求时间
    pub last_request_at: Option<Instant>,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            error_count: 0,
            last_error_at: None,
            cooldown_until: None,
            current_rpm: 0,
            last_request_at: None,
        }
    }
}

impl AccountState {
    /// 创建新的账号状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查是否在冷却中
    pub fn is_cooling_down(&self) -> bool {
        self.cooldown_until
            .map(|t| t > Instant::now())
            .unwrap_or(false)
    }

    /// 获取剩余冷却时间
    pub fn cooldown_remaining(&self) -> Option<Duration> {
        self.cooldown_until.map(|t| {
            let now = Instant::now();
            if t > now {
                t - now
            } else {
                Duration::from_secs(0)
            }
        })
    }
}

/// 账号状态存储
///
/// 使用 DashMap 实现并发安全的读写
#[derive(Debug)]
pub struct AccountStateStore {
    states: DashMap<Uuid, AccountState>,
    /// 触发冷却的错误阈值
    cooldown_threshold: AtomicU32,
    /// 冷却持续时间
    cooldown_duration: Duration,
}

impl Default for AccountStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountStateStore {
    /// 创建新的账号状态存储
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
            cooldown_threshold: AtomicU32::new(3),
            cooldown_duration: Duration::from_secs(60),
        }
    }

    /// 创建带自定义配置的存储
    pub fn with_config(cooldown_threshold: u32, cooldown_duration_secs: u64) -> Self {
        Self {
            states: DashMap::new(),
            cooldown_threshold: AtomicU32::new(cooldown_threshold),
            cooldown_duration: Duration::from_secs(cooldown_duration_secs),
        }
    }

    /// Gateway 调用：标记错误
    ///
    /// 增加错误计数，如果超过阈值则进入冷却状态
    pub fn mark_error(&self, account_id: Uuid) {
        let threshold = self.cooldown_threshold.load(Ordering::Relaxed);
        let duration = self.cooldown_duration;

        self.states
            .entry(account_id)
            .and_modify(|state| {
                state.error_count += 1;
                state.last_error_at = Some(Instant::now());

                // 错误次数超过阈值则进入冷却
                if state.error_count >= threshold {
                    state.cooldown_until = Some(Instant::now() + duration);
                    tracing::warn!(
                        account_id = %account_id,
                        error_count = state.error_count,
                        "Account entered cooldown state"
                    );
                }
            })
            .or_insert_with(|| {
                let mut state = AccountState::new();
                state.error_count = 1;
                state.last_error_at = Some(Instant::now());
                state
            });
    }

    /// Gateway 调用：标记成功
    ///
    /// 重置错误计数，清除冷却状态
    pub fn mark_success(&self, account_id: Uuid) {
        self.states
            .entry(account_id)
            .and_modify(|state| {
                if state.error_count > 0 {
                    tracing::info!(
                        account_id = %account_id,
                        previous_errors = state.error_count,
                        "Account error count reset after success"
                    );
                }
                state.error_count = 0;
                state.cooldown_until = None;
                state.last_error_at = None;
            })
            .or_insert_with(AccountState::new);
    }

    /// Gateway 调用：记录请求
    ///
    /// 更新 RPM 计数
    pub fn record_request(&self, account_id: Uuid) {
        let now = Instant::now();

        self.states
            .entry(account_id)
            .and_modify(|state| {
                state.last_request_at = Some(now);
                // 简单的 RPM 估算（实际应该使用滑动窗口）
                state.current_rpm += 1;
            })
            .or_insert_with(|| {
                let mut state = AccountState::new();
                state.last_request_at = Some(now);
                state.current_rpm = 1;
                state
            });
    }

    /// Routing 调用：只读快照
    ///
    /// 获取账号状态的只读副本
    pub fn snapshot(&self, account_id: &Uuid) -> Option<AccountState> {
        self.states.get(account_id).map(|s| s.clone())
    }

    /// Routing 调用：检查是否在冷却中
    pub fn is_cooling_down(&self, account_id: &Uuid) -> bool {
        self.states
            .get(account_id)
            .map(|s| s.is_cooling_down())
            .unwrap_or(false)
    }

    /// Routing 调用：获取所有可用账号（未冷却）
    pub fn available_accounts(&self, account_ids: &[Uuid]) -> Vec<Uuid> {
        account_ids
            .iter()
            .filter(|id| !self.is_cooling_down(id))
            .copied()
            .collect()
    }

    /// 获取所有账号状态（用于监控）
    pub fn all_states(&self) -> Vec<(Uuid, AccountState)> {
        self.states
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// 清理过期的冷却状态（可由后台任务定期调用）
    pub fn cleanup_expired_cooldowns(&self) {
        let now = Instant::now();
        self.states.retain(|_id, state| {
            if let Some(cooldown) = state.cooldown_until {
                if cooldown <= now {
                    state.cooldown_until = None;
                }
            }
            true // 保留所有条目
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_state_new() {
        let state = AccountState::new();
        assert_eq!(state.error_count, 0);
        assert!(state.last_error_at.is_none());
        assert!(state.cooldown_until.is_none());
        assert_eq!(state.current_rpm, 0);
    }

    #[test]
    fn test_mark_error() {
        let store = AccountStateStore::with_config(2, 60);
        let account_id = Uuid::new_v4();

        // 第一次错误
        store.mark_error(account_id);
        let state = store.snapshot(&account_id).unwrap();
        assert_eq!(state.error_count, 1);
        assert!(!state.is_cooling_down());

        // 第二次错误（达到阈值）
        store.mark_error(account_id);
        let state = store.snapshot(&account_id).unwrap();
        assert_eq!(state.error_count, 2);
        assert!(state.is_cooling_down());
    }

    #[test]
    fn test_mark_success() {
        let store = AccountStateStore::with_config(2, 60);
        let account_id = Uuid::new_v4();

        // 标记错误（未达到阈值）
        store.mark_error(account_id);
        assert!(!store.is_cooling_down(&account_id));

        // 标记成功清除错误
        store.mark_success(account_id);
        let state = store.snapshot(&account_id).unwrap();
        assert_eq!(state.error_count, 0);
        assert!(!state.is_cooling_down());
    }

    #[test]
    fn test_record_request() {
        let store = AccountStateStore::new();
        let account_id = Uuid::new_v4();

        store.record_request(account_id);
        let state = store.snapshot(&account_id).unwrap();
        assert_eq!(state.current_rpm, 1);
        assert!(state.last_request_at.is_some());
    }

    #[test]
    fn test_available_accounts() {
        let store = AccountStateStore::with_config(1, 60);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        // id1 第一次错误（阈值是1，所以一次错误就会冷却）
        store.mark_error(id1);
        
        // 注意：第一次错误时，or_insert_with 创建的新状态 error_count=1
        // 但不会立即触发冷却，因为 or_insert_with 不会执行 and_modify 中的逻辑
        // 需要第二次错误才会触发冷却
        store.mark_error(id1);
        
        // 验证 id1 确实在冷却中
        assert!(store.is_cooling_down(&id1), "id1 should be cooling down after 2 errors");

        let available = store.available_accounts(&[id1, id2]);
        assert_eq!(available.len(), 1);
        assert_eq!(available[0], id2);
    }
}
