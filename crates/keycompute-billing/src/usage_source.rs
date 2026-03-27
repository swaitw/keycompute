//! 用量来源
//!
//! 定义用量数据的来源优先级

/// 用量数据来源
#[derive(Debug, Clone, PartialEq, Default)]
pub enum UsageSource {
    /// Provider 报告的用量（优先级最高）
    ProviderReported,
    /// Gateway 累积的用量（当 Provider 未报告时使用）
    #[default]
    GatewayAccumulated,
}

impl UsageSource {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            UsageSource::ProviderReported => "provider_reported",
            UsageSource::GatewayAccumulated => "gateway_accumulated",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "provider_reported" => Some(UsageSource::ProviderReported),
            "gateway_accumulated" => Some(UsageSource::GatewayAccumulated),
            _ => None,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        Self::parse(s)
    }

    /// 获取优先级（数值越小优先级越高）
    pub fn priority(&self) -> u8 {
        match self {
            UsageSource::ProviderReported => 1,
            UsageSource::GatewayAccumulated => 2,
        }
    }
}

/// 选择最佳用量来源
///
/// 优先级：ProviderReported > GatewayAccumulated
pub fn select_best_source(
    provider_reported: Option<(u32, u32)>,
    gateway_accumulated: (u32, u32),
) -> (UsageSource, u32, u32) {
    if let Some((input, output)) = provider_reported {
        (UsageSource::ProviderReported, input, output)
    } else {
        (
            UsageSource::GatewayAccumulated,
            gateway_accumulated.0,
            gateway_accumulated.1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_source_as_str() {
        assert_eq!(UsageSource::ProviderReported.as_str(), "provider_reported");
        assert_eq!(
            UsageSource::GatewayAccumulated.as_str(),
            "gateway_accumulated"
        );
    }

    #[test]
    fn test_usage_source_from_str() {
        assert_eq!(
            UsageSource::from_str("provider_reported"),
            Some(UsageSource::ProviderReported)
        );
        assert_eq!(
            UsageSource::from_str("gateway_accumulated"),
            Some(UsageSource::GatewayAccumulated)
        );
        assert_eq!(UsageSource::from_str("unknown"), None);
    }

    #[test]
    fn test_usage_source_priority() {
        assert!(
            UsageSource::ProviderReported.priority() < UsageSource::GatewayAccumulated.priority()
        );
    }

    #[test]
    fn test_select_best_source_provider_first() {
        let provider = Some((100, 200));
        let gateway = (150, 250);

        let (source, input, output) = select_best_source(provider, gateway);
        assert_eq!(source, UsageSource::ProviderReported);
        assert_eq!(input, 100);
        assert_eq!(output, 200);
    }

    #[test]
    fn test_select_best_source_fallback_to_gateway() {
        let provider: Option<(u32, u32)> = None;
        let gateway = (150, 250);

        let (source, input, output) = select_best_source(provider, gateway);
        assert_eq!(source, UsageSource::GatewayAccumulated);
        assert_eq!(input, 150);
        assert_eq!(output, 250);
    }
}
