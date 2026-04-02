use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Pagination, Table, TableHead};

use crate::router::Route;
use crate::services::{api_client::with_auto_refresh, billing_service, payment_service};
use crate::stores::auth_store::AuthStore;
use crate::utils::time::format_time;

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

    // 用量统计（真实数据，来自 usage_logs 表）
    let usage_stats = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            billing_service::stats(&token).await
        })
        .await
    });

    // 用量明细（真实数据，来自 usage_logs 表）
    let usage_records = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            billing_service::list(&token).await
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
                            p { class: "stat-value", "¥ {b.available_balance}" }
                        },
                    }
                }
                div {
                    class: "stat-card",
                    p { class: "stat-title", "冻结金额" }
                    match balance() {
                        Some(Ok(b)) => rsx! { p { class: "stat-value", "¥ {b.frozen_balance}" } },
                        _ => rsx! { p { class: "stat-value", "—" } },
                    }
                }
                div {
                    class: "stat-card",
                    p { class: "stat-title", "总充值" }
                    match balance() {
                        Some(Ok(b)) => rsx! { p { class: "stat-value", "¥ {b.total_recharged}" } },
                        _ => rsx! { p { class: "stat-value", "—" } },
                    }
                }
                div {
                    class: "stat-card",
                    p { class: "stat-title", "总消耗" }
                    match balance() {
                        Some(Ok(b)) => rsx! { p { class: "stat-value", "¥ {b.total_consumed}" } },
                        _ => rsx! { p { class: "stat-value", "—" } },
                    }
                }
                match usage_stats() {
                    Some(Ok(s)) => rsx! {
                        div { class: "stat-card",
                            p { class: "stat-title", "用量请求数" }
                            p { class: "stat-value", "{s.total_requests}" }
                        }
                        div { class: "stat-card",
                            p { class: "stat-title", "输入Tokens" }
                            p { class: "stat-value", "{s.input_tokens}" }
                        }
                        div { class: "stat-card",
                            p { class: "stat-title", "输出Tokens" }
                            p { class: "stat-value", "{s.output_tokens}" }
                        }
                        div { class: "stat-card",
                            p { class: "stat-title", "总费用" }
                            p { class: "stat-value", "¥{s.total_cost:.2}" }
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
                                            TableHead { "主题" }
                                            TableHead { "状态" }
                                            TableHead { "时间" }
                                        }
                                    }
                                    tbody {
                                        for order in list.iter() {
                                            tr {
                                                key: "{order.id}",
                                                td { code { "{order.out_trade_no}" } }
                                                td { "¥ {order.amount}" }
                                                td { "{order.subject}" }
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

            // ─── 用量明细 ───
            div { class: "section",
                h2 { class: "section-title", "用量明细" }
                match usage_records() {
                    None => rsx! { p { class: "loading-text", "加载中..." } },
                    Some(Err(e)) => rsx! { p { class: "error-text", "加载失败：{e}" } },
                    Some(Ok(recs)) if recs.is_empty() => rsx! {
                        p { class: "empty-text", "暂无用量记录" }
                    },
                    Some(Ok(recs)) => rsx! {
                        div { class: "table-container",
                            table { class: "data-table",
                                thead {
                                    tr {
                                        th { "时间" }
                                        th { "模型" }
                                        th { "输入Tokens" }
                                        th { "输出Tokens" }
                                        th { "总Tokens" }
                                        th { "费用" }
                                        th { "状态" }
                                    }
                                }
                                tbody {
                                    {
                                        let start = (page() as usize - 1) * PAGE_SIZE;
                                        rsx! {
                                            for r in recs.iter().skip(start).take(PAGE_SIZE) {
                                                tr {
                                                    td { { format_time(&r.created_at) } }
                                                    td { "{r.model}" }
                                                    td { "{r.prompt_tokens}" }
                                                    td { "{r.completion_tokens}" }
                                                    td { "{r.total_tokens}" }
                                                    td { "¥{r.cost:.2}" }
                                                    td {
                                                        span {
                                                            class: if r.status == "success" { "badge badge-success" } else { "badge badge-warning" },
                                                            "{r.status}"
                                                        }
                                                    }
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
