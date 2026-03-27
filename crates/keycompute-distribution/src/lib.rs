//! Distribution Module
//!
//! 二级分销模块，从 usage_logs 派生。
//! 架构约束：Billing 完成后触发，不修改主账单，不影响执行链路。

pub mod calculator;
pub mod records;
pub mod rule;

pub use calculator::{DistributionShare, calculate_shares};
pub use records::{DistributionRecord, DistributionService};
pub use rule::{DistributionRule, RuleEngine};

use rust_decimal::Decimal;
use uuid::Uuid;

/// 分销上下文
#[derive(Debug, Clone)]
pub struct DistributionContext {
    /// 主账单 ID（usage_log_id）
    pub usage_log_id: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 用户应付金额
    pub user_amount: Decimal,
    /// 货币
    pub currency: String,
}

impl DistributionContext {
    /// 创建新的分销上下文
    pub fn new(
        usage_log_id: Uuid,
        tenant_id: Uuid,
        user_amount: Decimal,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            usage_log_id,
            tenant_id,
            user_amount,
            currency: currency.into(),
        }
    }
}

/// 分销层级（二级分销）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistributionLevel {
    /// 第一级（直接推荐人）
    Level1,
    /// 第二级（间接推荐人）
    Level2,
}

impl DistributionLevel {
    /// 获取层级名称
    pub fn as_str(&self) -> &'static str {
        match self {
            DistributionLevel::Level1 => "level1",
            DistributionLevel::Level2 => "level2",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "level1" => Some(DistributionLevel::Level1),
            "level2" => Some(DistributionLevel::Level2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_distribution_context() {
        let usage_log_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let amount = Decimal::from(100);

        let ctx = DistributionContext::new(usage_log_id, tenant_id, amount, "CNY");

        assert_eq!(ctx.usage_log_id, usage_log_id);
        assert_eq!(ctx.tenant_id, tenant_id);
        assert_eq!(ctx.user_amount, amount);
        assert_eq!(ctx.currency, "CNY");
    }

    #[test]
    fn test_distribution_level() {
        assert_eq!(DistributionLevel::Level1.as_str(), "level1");
        assert_eq!(DistributionLevel::Level2.as_str(), "level2");

        assert_eq!(
            DistributionLevel::parse("level1"),
            Some(DistributionLevel::Level1)
        );
        assert_eq!(
            DistributionLevel::parse("level2"),
            Some(DistributionLevel::Level2)
        );
        assert_eq!(DistributionLevel::parse("level3"), None);
    }
}
