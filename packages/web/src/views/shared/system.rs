use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Table, TableHead};

use crate::services::{api_client::with_auto_refresh, debug_service};
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::UserStore;
use crate::views::shared::accounts::NoPermissionView;

/// 系统诊断页面（仅 Admin 可访问）
///
/// 展示 Provider 健康状态、网关运行统计、路由调试信息（调用 DebugApi）
#[component]
pub fn System() -> Element {
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();
    let is_admin = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.is_admin())
        .unwrap_or(false);

    if !is_admin {
        return rsx! { NoPermissionView { resource: "系统诊断" } };
    }

    let provider_health = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            debug_service::provider_health(&token).await
        })
        .await
    });

    let gateway_stats = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            debug_service::gateway_stats(&token).await
        })
        .await
    });

    let routing_info = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            // 使用默认模型进行路由调试
            debug_service::routing("gpt-4o", &token).await
        })
        .await
    });

    let (total_req, success_rate, avg_latency, active_conns) = match gateway_stats() {
        Some(Ok(ref s)) => (
            s.total_requests.to_string(),
            format!(
                "{:.1}%",
                s.successful_requests as f64 / s.total_requests.max(1) as f64 * 100.0
            ),
            format!("{:.0}ms", s.average_latency_ms),
            s.active_connections.to_string(),
        ),
        Some(Err(_)) => ("加载失败".into(), "—".into(), "—".into(), "—".into()),
        None => ("加载中...".into(), "—".into(), "—".into(), "—".into()),
    };

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "系统诊断" }
            p { class: "page-description", "查看 Provider 健康状态、网关运行统计和路由调试信息" }
        }

        // Provider 健康状态
        div { class: "section",
            h2 { class: "section-title", "Provider 健康状态" }
            div { class: "card",
                div { class: "card-body",
                    match provider_health() {
                        None => rsx! { p { class: "text-secondary", "加载中..." } },
                        Some(Err(_)) => rsx! { p { class: "text-secondary", "加载失败" } },
                        Some(Ok(ref resp)) => rsx! {
                            div { class: "health-grid",
                                for (name, health) in resp.providers.iter() {
                                    HealthItem {
                                        name: name.clone(),
                                        status: health.status.clone(),
                                        latency_ms: health.latency_ms,
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }

        // 网关运行统计
        div { class: "section",
            h2 { class: "section-title", "网关运行统计" }
            div { class: "stats-grid",
                div { class: "stat-card card",
                    div { class: "card-body",
                        p { class: "stat-label", "总请求数" }
                        p { class: "stat-value", "{total_req}" }
                    }
                }
                div { class: "stat-card card",
                    div { class: "card-body",
                        p { class: "stat-label", "成功率" }
                        p { class: "stat-value", "{success_rate}" }
                    }
                }
                div { class: "stat-card card",
                    div { class: "card-body",
                        p { class: "stat-label", "平均响应时间" }
                        p { class: "stat-value", "{avg_latency}" }
                    }
                }
                div { class: "stat-card card",
                    div { class: "card-body",
                        p { class: "stat-label", "活跃连接数" }
                        p { class: "stat-value", "{active_conns}" }
                    }
                }
            }
        }

        // 路由调试
        div { class: "section",
            h2 { class: "section-title", "路由调试" }
            div { class: "card",
                div { class: "card-header",
                    h3 { class: "card-title", "Provider 状态诊断" }
                }
                div { class: "card-body",
                    match routing_info() {
                        None => rsx! { p { class: "text-secondary", "加载中..." } },
                        Some(Err(ref e)) => rsx! {
                            div { class: "alert alert-error",
                                p { "加载失败: {e}" }
                            }
                        },
                        Some(Ok(ref info)) => rsx! {
                            div {
                                // 路由结果
                                if info.routed {
                                    div { class: "alert alert-success",
                                        p { "✓ 路由成功" }
                                        if let Some(ref primary) = info.primary {
                                            p { class: "text-sm",
                                                "主目标: {primary.provider} ({primary.endpoint})"
                                            }
                                        }
                                        if !info.fallback_chain.is_empty() {
                                            p { class: "text-sm",
                                                "备用链路: {info.fallback_chain.len()} 个"
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "alert alert-warning",
                                        p { "✗ 路由失败" }
                                        if let Some(ref msg) = info.message {
                                            p { class: "text-sm", "{msg}" }
                                        }
                                    }
                                }

                                // Provider 状态表格
                                h4 { class: "subsection-title", "Provider 状态" }
                                Table {
                                    empty: info.provider_status.is_empty(),
                                    empty_text: "未配置任何 Provider".to_string(),
                                    col_count: 4,
                                    thead {
                                        tr {
                                            TableHead { "Provider" }
                                            TableHead { "健康状态" }
                                            TableHead { "账号数量" }
                                            TableHead { "状态" }
                                        }
                                    }
                                    tbody {
                                        for ps in info.provider_status.iter() {
                                            tr {
                                                td { "{ps.provider}" }
                                                td {
                                                    if ps.is_healthy {
                                                        Badge { variant: BadgeVariant::Success, "健康" }
                                                    } else {
                                                        Badge { variant: BadgeVariant::Error, "不健康" }
                                                    }
                                                }
                                                td { "{ps.account_count}" }
                                                td { "{ps.status}" }
                                            }
                                        }
                                    }
                                }

                                // 定价信息
                                h4 { class: "subsection-title", "定价信息" }
                                div { class: "info-grid",
                                    div { class: "info-item",
                                        span { class: "info-label", "模型" }
                                        span { class: "info-value", "{info.pricing.model_name}" }
                                    }
                                    div { class: "info-item",
                                        span { class: "info-label", "货币" }
                                        span { class: "info-value", "{info.pricing.currency}" }
                                    }
                                    div { class: "info-item",
                                        span { class: "info-label", "输入价格" }
                                        span { class: "info-value", "{info.pricing.input_price_per_1k} / 1K tokens" }
                                    }
                                    div { class: "info-item",
                                        span { class: "info-label", "输出价格" }
                                        span { class: "info-value", "{info.pricing.output_price_per_1k} / 1K tokens" }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

// ── 内部组件 ──────────────────────────────────────────────────────

#[component]
fn HealthItem(name: String, status: String, latency_ms: Option<i64>) -> Element {
    let (status_class, status_text) = match status.as_str() {
        "healthy" => (BadgeVariant::Success, "健康"),
        "degraded" => (BadgeVariant::Warning, "降级"),
        "unhealthy" => (BadgeVariant::Error, "异常"),
        _ => (BadgeVariant::Neutral, "未知"),
    };

    rsx! {
        div { class: "health-item",
            div { class: "health-name", "{name}" }
            div { class: "health-status",
                Badge { variant: status_class, "{status_text}" }
                if let Some(ms) = latency_ms {
                    span { class: "text-secondary text-sm", "{ms}ms" }
                }
            }
        }
    }
}
