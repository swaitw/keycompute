use dioxus::prelude::*;

use crate::router::Route;
use crate::services::billing_service;
use crate::stores::auth_store::AuthStore;

/// 账单页面 - /billing
#[component]
pub fn Billing() -> Element {
    let auth_store = use_context::<AuthStore>();
    let nav = use_navigator();

    // 账单统计
    let stats = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        billing_service::stats(&token).await
    });

    // 账单记录
    let records = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        billing_service::list(None, &token).await
    });

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "账单" }
                p { class: "page-subtitle", "查看账单记录与支付历史" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| { nav.push(Route::Recharge {}); },
                    "充值"
                }
            }

            // 汇总卡片
            div { class: "stats-grid",
                match stats() {
                    None => rsx! { p { "加载中..." } },
                    Some(Err(e)) => rsx! { p { "加载失败：{e}" } },
                    Some(Ok(s)) => rsx! {
                        div { class: "stat-card",
                            div { class: "stat-body",
                                p { class: "stat-title", "总金额" }
                                p { class: "stat-value", "{s.total_amount:.2} {s.currency}" }
                                p { class: "stat-label", "统计周期：{s.period}" }
                            }
                        }
                        div { class: "stat-card",
                            div { class: "stat-body",
                                p { class: "stat-title", "已支付" }
                                p { class: "stat-value", "{s.total_paid:.2} {s.currency}" }
                                p { class: "stat-label", "已完成订单" }
                            }
                        }
                        div { class: "stat-card",
                            div { class: "stat-body",
                                p { class: "stat-title", "待支付" }
                                p { class: "stat-value", "{s.total_unpaid:.2} {s.currency}" }
                                p { class: "stat-label", "未完成订单" }
                            }
                        }
                    },
                }
            }

            // 账单记录表格
            div { class: "section",
                h2 { class: "section-title", "账单记录" }
                match records() {
                    None => rsx! { p { class: "loading-text", "加载中..." } },
                    Some(Err(e)) => rsx! { p { class: "error-text", "加载失败：{e}" } },
                    Some(Ok(recs)) if recs.is_empty() => rsx! {
                        p { class: "empty-text", "暂无账单记录" }
                    },
                    Some(Ok(recs)) => rsx! {
                        div { class: "table-container",
                            table { class: "data-table",
                                thead {
                                    tr {
                                        th { "时间" }
                                        th { "金额" }
                                        th { "币种" }
                                        th { "描述" }
                                        th { "状态" }
                                        th { "支付时间" }
                                    }
                                }
                                tbody {
                                    for r in recs {
                                        tr {
                                            td { "{r.created_at}" }
                                            td { "{r.amount:.4}" }
                                            td { "{r.currency}" }
                                            td { { r.description.as_deref().unwrap_or("—") } }
                                            td {
                                                span {
                                                    class: if r.status == "paid" { "badge badge-success" } else { "badge badge-warning" },
                                                    "{r.status}"
                                                }
                                            }
                                            td { { r.paid_at.as_deref().unwrap_or("—") } }
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
