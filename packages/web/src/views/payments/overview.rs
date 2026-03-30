use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Pagination, Table, TableHead};

use crate::router::Route;
use crate::services::{api_client::with_auto_refresh, billing_service, payment_service};
use crate::stores::auth_store::AuthStore;
use crate::utils::time::{format_time, format_time_opt};

const PAGE_SIZE: usize = 20;

/// 支付与账单页面 - /payments
///
/// 包含：账户余额、充値记录、账单统计、账单明细
#[component]
pub fn PaymentsOverview() -> Element {
    let auth_store = use_context::<AuthStore>();

    let nav = use_navigator();
    let mut page = use_signal(|| 1u32);

    let balance = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            payment_service::get_balance(&token).await
        })
        .await
    });

    let orders = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            payment_service::list_orders(None, &token).await
        })
        .await
    });

    // 账单统计
    let billing_stats = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            billing_service::stats(&token).await
        })
        .await
    });

    // 账单明细
    let billing_records = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            billing_service::list(None, &token).await
        })
        .await
    });

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "支付与账单" }
                p { class: "page-subtitle", "查看账户余额、充値记录与账单明细" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| { nav.push(Route::Recharge {}); },
                    "立即充値"
                }
            }

            // ─── 账户余额 ───
            div { class: "stats-grid",
                div {
                    class: "stat-card",
                    p { class: "stat-title", "账户余额" }
                    match balance() {
                        None => rsx! { p { class: "stat-value", "加载中..." } },
                        Some(Err(e)) => rsx! { p { class: "stat-value text-error", "错误: {e}" } },
                        Some(Ok(b)) => rsx! {
                            p { class: "stat-value", "¥ {b.balance:.2}" }
                            p { class: "stat-label", "{b.currency}" }
                        },
                    }
                }
                div {
                    class: "stat-card",
                    p { class: "stat-title", "冻结金额" }
                    match balance() {
                        Some(Ok(b)) => rsx! { p { class: "stat-value", "¥ {b.frozen_balance:.2}" } },
                        _ => rsx! { p { class: "stat-value", "—" } },
                    }
                }
                match billing_stats() {
                    Some(Ok(s)) => rsx! {
                        div { class: "stat-card",
                            p { class: "stat-title", "账单总金额" }
                            p { class: "stat-value", "{s.total_amount:.2} {s.currency}" }
                            p { class: "stat-label", "统计周期：{s.period}" }
                        }
                        div { class: "stat-card",
                            p { class: "stat-title", "已消耗" }
                            p { class: "stat-value", "{s.total_paid:.2} {s.currency}" }
                            p { class: "stat-label", "已完成订单" }
                        }
                        div { class: "stat-card",
                            p { class: "stat-title", "待支付" }
                            p { class: "stat-value", "{s.total_unpaid:.2} {s.currency}" }
                            p { class: "stat-label", "未完成订单" }
                        }
                    },
                    _ => rsx! {},
                }
            }

            // ─── 充値记录 ───
            div { class: "section",
                h2 { class: "section-title", "充値记录" }
                match orders() {
                    None => rsx! { div { class: "loading-state", "加载中..." } },
                    Some(Err(e)) => rsx! { div { class: "alert alert-error", "加载失败：{e}" } },
                    Some(Ok(list)) => {
                        if list.is_empty() {
                            rsx! { div { class: "empty-state", p { "暂无充値记录" } } }
                        } else {
                            rsx! {
                                Table {
                                    col_count: 4,
                                    thead {
                                        tr {
                                            TableHead { "订单号" }
                                            TableHead { "金额" }
                                            TableHead { "状态" }
                                            TableHead { "时间" }
                                        }
                                    }
                                    tbody {
                                        for order in list.iter() {
                                            tr {
                                                key: "{order.id}",
                                                td { code { "{order.out_trade_no}" } }
                                                td { "¥ {order.amount:.2}" }
                                                td {
                                                    Badge {
                                                        variant: payment_status_variant(&order.status),
                                                        "{order.status}"
                                                    }
                                                }
                                                td { { format_time(&order.created_at) } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ─── 账单明细 ───
            div { class: "section",
                h2 { class: "section-title", "账单明细" }
                match billing_records() {
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
                                    {
                                        let start = (page() as usize - 1) * PAGE_SIZE;
                                        rsx! {
                                            for r in recs.iter().skip(start).take(PAGE_SIZE) {
                                                tr {
                                                    td { { format_time(&r.created_at) } }
                                                    td { "{r.amount:.4}" }
                                                    td { "{r.currency}" }
                                                    td { { r.description.as_deref().unwrap_or("—") } }
                                                    td {
                                                        span {
                                                            class: if r.status == "paid" { "badge badge-success" } else { "badge badge-warning" },
                                                            "{r.status}"
                                                        }
                                                    }
                                                    td { { format_time_opt(r.paid_at.as_deref()) } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        {
                            let total = recs.len();
                            let total_pages = total.div_ceil(PAGE_SIZE).max(1) as u32;
                            rsx! {
                                div { class: "pagination",
                                    span { class: "pagination-info", "共 {total} 条" }
                                    Pagination {
                                        current: page(),
                                        total_pages,
                                        on_page_change: move |p| page.set(p),
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

fn payment_status_variant(status: &str) -> BadgeVariant {
    match status {
        "paid" | "success" => BadgeVariant::Success,
        "pending" | "processing" => BadgeVariant::Warning,
        "failed" | "cancelled" => BadgeVariant::Error,
        _ => BadgeVariant::Neutral,
    }
}
