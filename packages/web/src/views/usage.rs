use dioxus::prelude::*;
use ui::{LineChart, LineSeriesData};

use crate::services::usage_service;
use crate::stores::auth_store::AuthStore;
use std::collections::HashMap;

/// 用量统计页面 - /usage
#[component]
pub fn Usage() -> Element {
    let auth_store = use_context::<AuthStore>();

    // 汇总统计
    let stats = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        usage_service::stats(&token).await
    });

    // 明细记录（最近 50 条）
    let records = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        usage_service::list(
            Some(client_api::api::usage::UsageQueryParams::new().with_limit(50)),
            &token,
        )
        .await
    });

    // 折线图：按日期聚合调用次数
    let (chart_x, chart_series) = match records() {
        Some(Ok(ref recs)) => {
            let mut by_date: HashMap<String, f64> = HashMap::new();
            for r in recs {
                let date = r.created_at.get(..10).unwrap_or("").to_string();
                *by_date.entry(date).or_default() += 1.0;
            }
            let mut pairs: Vec<(String, f64)> = by_date.into_iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            let x: Vec<String> = pairs.iter().map(|(d, _)| d.clone()).collect();
            let y: Vec<f64> = pairs.iter().map(|(_, v)| *v).collect();
            (
                x,
                vec![LineSeriesData {
                    name: "调用次数".to_string(),
                    data: y,
                }],
            )
        }
        _ => (vec![], vec![]),
    };

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "用量统计" }
                p { class: "page-subtitle", "查看 API 调用记录与 Token 消耗" }
            }

            // 汇总卡片
            div { class: "stats-grid",
                match stats() {
                    None => rsx! { p { "加载中..." } },
                    Some(Err(e)) => rsx! { p { "加载失败：{e}" } },
                    Some(Ok(s)) => rsx! {
                        div { class: "stat-card",
                            div { class: "stat-body",
                                p { class: "stat-title", "总调用次数" }
                                p { class: "stat-value", "{s.total_requests}" }
                                p { class: "stat-label", "统计周期：{s.period}" }
                            }
                        }
                        div { class: "stat-card",
                            div { class: "stat-body",
                                p { class: "stat-title", "总 Token 数" }
                                p { class: "stat-value", "{s.total_tokens}" }
                                p { class: "stat-label",
                                    "提示词：{s.total_prompt_tokens} / 补全：{s.total_completion_tokens}"
                                }
                            }
                        }
                        div { class: "stat-card",
                            div { class: "stat-body",
                                p { class: "stat-title", "累计费用" }
                                p { class: "stat-value", "¥{s.total_cost:.4}" }
                                p { class: "stat-label", "按使用量计费" }
                            }
                        }
                    },
                }
            }

            // 调用趋势折线图
            if !chart_x.is_empty() {
                div { class: "section",
                    h2 { class: "section-title", "调用趋势" }
                    div { class: "chart-container",
                        LineChart {
                            id: "usage-line-chart",
                            title: "",
                            x_data: chart_x,
                            series: chart_series,
                            width: 800,
                            height: 300,
                        }
                    }
                }
            }

            // 明细记录表格
            div { class: "section",
                h2 { class: "section-title", "调用记录" }
                match records() {
                    None => rsx! { p { class: "loading-text", "加载中..." } },
                    Some(Err(e)) => rsx! { p { class: "error-text", "加载失败：{e}" } },
                    Some(Ok(recs)) if recs.is_empty() => rsx! {
                        p { class: "empty-text", "暂无记录" }
                    },
                    Some(Ok(recs)) => rsx! {
                        div { class: "table-container",
                            table { class: "data-table",
                                thead {
                                    tr {
                                        th { "时间" }
                                        th { "模型" }
                                        th { "提示词 Token" }
                                        th { "补全 Token" }
                                        th { "总 Token" }
                                        th { "费用" }
                                    }
                                }
                                tbody {
                                    for r in recs {
                                        tr {
                                            td { "{r.created_at}" }
                                            td { "{r.model}" }
                                            td { "{r.prompt_tokens}" }
                                            td { "{r.completion_tokens}" }
                                            td { "{r.total_tokens}" }
                                            td {
                                                {
                                                    if let Some(c) = r.cost {
                                                        format!("¥{c:.6}")
                                                    } else {
                                                        "—".to_string()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}
