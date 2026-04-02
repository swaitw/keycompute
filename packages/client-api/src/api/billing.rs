//! 账单模块
//!
//! 处理账单记录查询和统计
//! 注意：使用 /api/v1/usage 和 /api/v1/usage/stats 获取真实用量数据
//! （/api/v1/billing/records 和 /api/v1/billing/stats 返回模拟数据）

use crate::client::ApiClient;
use crate::error::Result;
use serde::{Deserialize, Serialize};

use super::common::encode_query_value;

/// 账单 API 客户端
#[derive(Debug, Clone)]
pub struct BillingApi {
    client: ApiClient,
}

impl BillingApi {
    /// 创建新的账单 API 客户端
    pub fn new(client: &ApiClient) -> Self {
        Self {
            client: client.clone(),
        }
    }

    /// 获取用量记录列表（真实数据，从 usage_logs 表查询）
    ///
    /// 调用 /api/v1/usage 获取当前用户的真实用量记录
    pub async fn list_usage_records(&self, token: &str) -> Result<Vec<UsageRecord>> {
        self.client.get_json("/api/v1/usage", Some(token)).await
    }

    /// 获取用量统计（真实数据，从 usage_logs 表聚合）
    ///
    /// 调用 /api/v1/usage/stats 获取当前用户的真实用量统计
    pub async fn get_usage_stats(&self, token: &str) -> Result<UsageStats> {
        self.client
            .get_json("/api/v1/usage/stats", Some(token))
            .await
    }

    /// 获取账单记录列表（⚠️ 模拟数据，仅用于演示）
    #[deprecated(since = "0.1.0", note = "使用 list_usage_records 获取真实数据")]
    pub async fn list_billing_records(
        &self,
        params: Option<&BillingQueryParams>,
        token: &str,
    ) -> Result<Vec<BillingRecord>> {
        let path = if let Some(p) = params {
            format!("/api/v1/billing/records?{}", p.to_query_string())
        } else {
            "/api/v1/billing/records".to_string()
        };
        // 后端返回 { records: Vec<BillingRecord>, total: i64 }
        #[derive(Deserialize)]
        struct BillingListResponse {
            records: Vec<BillingRecord>,
            #[allow(dead_code)]
            total: i64,
        }
        let resp: BillingListResponse = self.client.get_json(&path, Some(token)).await?;
        Ok(resp.records)
    }

    /// 获取账单统计（⚠️ 模拟数据，仅用于演示）
    #[deprecated(since = "0.1.0", note = "使用 get_usage_stats 获取真实数据")]
    pub async fn get_billing_stats(&self, token: &str) -> Result<BillingStats> {
        self.client
            .get_json("/api/v1/billing/stats", Some(token))
            .await
    }
}

/// 账单查询参数
#[derive(Debug, Clone, Serialize, Default)]
pub struct BillingQueryParams {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl BillingQueryParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_start_date(mut self, date: impl Into<String>) -> Self {
        self.start_date = Some(date.into());
        self
    }

    pub fn with_end_date(mut self, date: impl Into<String>) -> Self {
        self.end_date = Some(date.into());
        self
    }

    pub fn with_limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: i32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref start) = self.start_date {
            params.push(format!("start_date={}", encode_query_value(start)));
        }
        if let Some(ref end) = self.end_date {
            params.push(format!("end_date={}", encode_query_value(end)));
        }
        if let Some(limit) = self.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = self.offset {
            params.push(format!("offset={}", offset));
        }
        params.join("&")
    }
}

/// 账单记录
#[derive(Debug, Clone, Deserialize)]
pub struct BillingRecord {
    pub id: String,
    #[serde(rename = "request_id")]
    pub user_id: String,
    #[serde(rename = "model_name")]
    pub model: String,
    pub provider_name: String,
    #[serde(rename = "input_tokens")]
    pub prompt_tokens: i32,
    #[serde(rename = "output_tokens")]
    pub completion_tokens: i32,
    #[serde(rename = "user_amount")]
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub created_at: String,
}

/// 账单统计中的模型统计
#[derive(Debug, Clone, Deserialize)]
pub struct ModelStats {
    pub model_name: String,
    pub request_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub amount: String,
}

/// 账单统计
#[derive(Debug, Clone, Deserialize)]
pub struct BillingStats {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    #[serde(rename = "total_amount")]
    pub total_cost: String,
    pub currency: String,
    pub by_model: Vec<ModelStats>,
}

/// 用量记录（真实数据，来自 usage_logs 表）
#[derive(Debug, Clone, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
    pub model: String,
    #[serde(rename = "input_tokens")]
    pub prompt_tokens: i32,
    #[serde(rename = "output_tokens")]
    pub completion_tokens: i32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: i32,
    pub cost: f64,
    pub status: String,
    pub created_at: String,
}

/// 用量统计（真实数据，来自 usage_logs 表聚合）
#[derive(Debug, Clone, Deserialize)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_tokens: i64,
    #[serde(rename = "total_input_tokens")]
    pub input_tokens: i64,
    #[serde(rename = "total_output_tokens")]
    pub output_tokens: i64,
    pub total_cost: f64,
    pub period: String,
}
