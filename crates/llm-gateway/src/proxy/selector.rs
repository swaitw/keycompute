//! 代理选择器
//!
//! 根据规则选择合适的代理出口

use std::collections::HashMap;
use uuid::Uuid;

/// 代理选择器
///
/// 支持多级代理选择：
/// 1. 精确匹配：`openai` -> `http://proxy1:8080`
/// 2. 通配符匹配：`*-cn` -> `http://cn-proxy:8080`
/// 3. 账号级代理：`(provider, account_id)` -> `http://proxy:8080`
#[derive(Debug, Clone, Default)]
pub struct ProxySelector {
    /// Provider 级代理映射
    provider_proxies: HashMap<String, String>,
    /// 账号级代理映射 (provider:account_id -> proxy_url)
    account_proxies: HashMap<String, String>,
    /// 通配符规则 (pattern -> proxy_url)
    pattern_proxies: Vec<(String, String)>,
}

impl ProxySelector {
    /// 创建新的代理选择器
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带预设代理的选择器
    pub fn with_proxies(proxies: HashMap<String, String>) -> Self {
        Self {
            provider_proxies: proxies,
            account_proxies: HashMap::new(),
            pattern_proxies: Vec::new(),
        }
    }

    /// 添加 Provider 级代理
    pub fn add_proxy(&mut self, provider: impl Into<String>, proxy_url: impl Into<String>) {
        self.provider_proxies
            .insert(provider.into(), proxy_url.into());
    }

    /// 添加账号级代理
    pub fn add_account_proxy(
        &mut self,
        provider: impl Into<String>,
        account_id: Uuid,
        proxy_url: impl Into<String>,
    ) {
        let key = format!("{}:{}", provider.into(), account_id);
        self.account_proxies.insert(key, proxy_url.into());
    }

    /// 添加通配符规则
    ///
    /// 支持的通配符：
    /// - `*` 匹配任意字符
    /// - `?` 匹配单个字符
    /// - `*-cn` 匹配以 `-cn` 结尾的 provider
    pub fn add_pattern(&mut self, pattern: impl Into<String>, proxy_url: impl Into<String>) {
        self.pattern_proxies
            .push((pattern.into(), proxy_url.into()));
    }

    /// 为 Provider 选择代理
    ///
    /// 选择优先级：
    /// 1. 精确匹配 provider_proxies
    /// 2. 通配符匹配 pattern_proxies
    pub fn select(&self, provider: &str) -> Option<&str> {
        // 1. 精确匹配
        if let Some(url) = self.provider_proxies.get(provider) {
            return Some(url);
        }

        // 2. 通配符匹配
        for (pattern, url) in &self.pattern_proxies {
            if Self::match_pattern(pattern, provider) {
                return Some(url);
            }
        }

        None
    }

    /// 为 Provider 和账号选择代理
    ///
    /// 选择优先级：
    /// 1. 账号级代理
    /// 2. Provider 级代理
    /// 3. 通配符匹配
    pub fn select_for_account(&self, provider: &str, account_id: Uuid) -> Option<&str> {
        // 1. 账号级代理
        let key = format!("{}:{}", provider, account_id);
        if let Some(url) = self.account_proxies.get(&key) {
            return Some(url);
        }

        // 2. Provider 级代理
        self.select(provider)
    }

    /// 通配符匹配
    fn match_pattern(pattern: &str, text: &str) -> bool {
        // 简单实现：支持 *-suffix 和 prefix-* 模式
        if pattern.starts_with("*-") {
            // *-cn 匹配以 -cn 结尾
            let suffix = &pattern[1..]; // -cn
            return text.ends_with(suffix);
        } else if pattern.ends_with("-*") {
            // openai-* 匹配以 openai- 开头
            let prefix = &pattern[..pattern.len() - 1]; // openai-
            return text.starts_with(prefix);
        } else if pattern.contains('*') {
            // 包含 * 的模式，使用简单的 glob 匹配
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let (prefix, suffix) = (parts[0], parts[1]);
                return text.starts_with(prefix) && text.ends_with(suffix);
            }
        }

        // 精确匹配
        pattern == text
    }

    /// 获取所有 Provider 代理
    pub fn provider_proxies(&self) -> &HashMap<String, String> {
        &self.provider_proxies
    }

    /// 获取所有账号代理
    pub fn account_proxies(&self) -> &HashMap<String, String> {
        &self.account_proxies
    }

    /// 清空所有代理配置
    pub fn clear(&mut self) {
        self.provider_proxies.clear();
        self.account_proxies.clear();
        self.pattern_proxies.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_exact_match() {
        let mut selector = ProxySelector::new();
        selector.add_proxy("openai", "http://proxy1:8080");
        selector.add_proxy("claude", "http://proxy2:8080");

        assert_eq!(selector.select("openai"), Some("http://proxy1:8080"));
        assert_eq!(selector.select("claude"), Some("http://proxy2:8080"));
        assert_eq!(selector.select("unknown"), None);
    }

    #[test]
    fn test_select_pattern_suffix() {
        let mut selector = ProxySelector::new();
        selector.add_pattern("*-cn", "http://cn-proxy:8080");

        assert_eq!(selector.select("openai-cn"), Some("http://cn-proxy:8080"));
        assert_eq!(selector.select("deepseek-cn"), Some("http://cn-proxy:8080"));
        assert_eq!(selector.select("openai"), None);
    }

    #[test]
    fn test_select_pattern_prefix() {
        let mut selector = ProxySelector::new();
        selector.add_pattern("openai-*", "http://openai-proxy:8080");

        assert_eq!(
            selector.select("openai-us"),
            Some("http://openai-proxy:8080")
        );
        assert_eq!(
            selector.select("openai-eu"),
            Some("http://openai-proxy:8080")
        );
        assert_eq!(selector.select("claude"), None);
    }

    #[test]
    fn test_select_account_proxy() {
        let mut selector = ProxySelector::new();
        selector.add_proxy("openai", "http://default-proxy:8080");

        let account_id = Uuid::nil();
        selector.add_account_proxy("openai", account_id, "http://account-proxy:8080");

        // 账号级代理优先
        assert_eq!(
            selector.select_for_account("openai", account_id),
            Some("http://account-proxy:8080")
        );

        // 其他账号使用 Provider 级代理
        let other_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        assert_eq!(
            selector.select_for_account("openai", other_id),
            Some("http://default-proxy:8080")
        );
    }

    #[test]
    fn test_priority() {
        let mut selector = ProxySelector::new();
        selector.add_proxy("openai", "http://provider-proxy:8080");
        selector.add_pattern("openai-*", "http://pattern-proxy:8080");

        // 精确匹配优先
        assert_eq!(
            selector.select("openai"),
            Some("http://provider-proxy:8080")
        );

        // 通配符匹配
        assert_eq!(
            selector.select("openai-us"),
            Some("http://pattern-proxy:8080")
        );
    }
}
