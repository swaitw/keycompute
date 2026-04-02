use client_api::error::Result;
use client_api::{
    BillingApi,
    api::billing::{UsageRecord, UsageStats},
};

use super::api_client::get_client;

/// 获取用量记录列表（真实数据，来自 usage_logs 表）
pub async fn list(token: &str) -> Result<Vec<UsageRecord>> {
    let client = get_client();
    BillingApi::new(&client).list_usage_records(token).await
}

/// 获取用量统计（真实数据，来自 usage_logs 表聚合）
pub async fn stats(token: &str) -> Result<UsageStats> {
    let client = get_client();
    BillingApi::new(&client).get_usage_stats(token).await
}
