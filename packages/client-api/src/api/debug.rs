//! 调试接口模块
//!
//! 提供路由调试、Provider 健康状态等调试功能（Admin）

use crate::client::ApiClient;
use crate::error::Result;
use serde::Deserialize;
use std::collections::HashMap;

/// 调试 API 客户端
#[derive(Debug, Clone)]
pub struct DebugApi {
    client: ApiClient,
}

impl DebugApi {
    /// 创建新的调试 API 客户端
    pub fn new(client: &ApiClient) -> Self {
        Self {
            client: client.clone(),
        }
    }

    /// 获取路由调试信息
    ///
    /// # 参数
    /// - `model`: 模型名称，用于模拟路由决策
    /// - `token`: 认证令牌
    pub async fn debug_routing(&self, model: &str, token: &str) -> Result<RoutingDebugInfo> {
        let path = format!("/debug/routing?model={}", urlencoding::encode(model));
        self.client.get_json(&path, Some(token)).await
    }

    /// 获取 Provider 健康状态
    pub async fn get_provider_health(&self, token: &str) -> Result<ProviderHealthResponse> {
        self.client.get_json("/debug/providers", Some(token)).await
    }

    /// 获取网关状态
    pub async fn get_gateway_status(&self, token: &str) -> Result<GatewayStatus> {
        self.client
            .get_json("/debug/gateway/status", Some(token))
            .await
    }

    /// 获取网关统计
    pub async fn get_gateway_stats(&self, token: &str) -> Result<GatewayStats> {
        self.client
            .get_json("/debug/gateway/stats", Some(token))
            .await
    }

    /// 检查 Provider 健康
    pub async fn check_provider_health(&self, token: &str) -> Result<HealthCheckResponse> {
        self.client
            .post_json("/debug/gateway/health", &serde_json::json!({}), Some(token))
            .await
    }
}

/// 路由调试信息
#[derive(Debug, Clone, Deserialize)]
pub struct RoutingDebugInfo {
    /// 请求 ID
    pub request_id: String,
    /// 是否成功路由
    pub routed: bool,
    /// 主目标（路由成功时有值）
    pub primary: Option<RoutingTargetInfo>,
    /// 备用链路
    pub fallback_chain: Vec<RoutingTargetInfo>,
    /// 定价信息
    pub pricing: PricingInfo,
    /// Provider 状态列表
    pub provider_status: Vec<ProviderStatusInfo>,
    /// 提示信息
    pub message: Option<String>,
}

/// 路由目标信息
#[derive(Debug, Clone, Deserialize)]
pub struct RoutingTargetInfo {
    pub provider: String,
    pub account_id: String,
    pub endpoint: String,
}

/// 定价信息
#[derive(Debug, Clone, Deserialize)]
pub struct PricingInfo {
    pub model_name: String,
    pub currency: String,
    pub input_price_per_1k: String,
    pub output_price_per_1k: String,
}

/// Provider 状态信息
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderStatusInfo {
    pub provider: String,
    pub is_healthy: bool,
    pub account_count: i64,
    pub status: String,
}

/// Provider 健康响应
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderHealthResponse {
    pub providers: HashMap<String, ProviderHealth>,
}

/// Provider 健康状态
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderHealth {
    pub status: String,
    pub last_check: Option<String>,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
}

/// 网关状态
#[derive(Debug, Clone, Deserialize)]
pub struct GatewayStatus {
    pub status: String,
    pub uptime_seconds: i64,
    pub version: String,
}

/// 网关统计
#[derive(Debug, Clone, Deserialize)]
pub struct GatewayStats {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub average_latency_ms: f64,
    pub active_connections: i32,
}

/// 健康检查响应
#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckResponse {
    pub checked_providers: Vec<String>,
    pub healthy_providers: Vec<String>,
    pub unhealthy_providers: Vec<String>,
}
