use client_api::{AdminApi, api::admin::PaymentQueryParams as AdminPaymentQueryParams};
use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Pagination, Table, TableHead};

const PAGE_SIZE: usize = 20;

use crate::services::{
    api_client::{get_client, with_auto_refresh},
    payment_service,
};
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::UserStore;
use crate::utils::time::format_time;

/// 支付订单页面
///
/// - 普通用户：仅查看自己的订单
/// - Admin：查看所有订单，支持审核和退款
#[component]
pub fn PaymentOrders() -> Element {
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();
    let is_admin = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.is_admin())
        .unwrap_or(false);

    let mut status_filter = use_signal(|| "all".to_string());
    let mut page = use_signal(|| 1u32);

    // 普通用户订单
    let my_orders = use_resource(move || async move {
        if is_admin {
            return Ok(vec![]);
        }
        let status = status_filter();
        let params = if status == "all" {
            None
        } else {
            Some(client_api::api::payment::PaymentQueryParams::default())
        };
        with_auto_refresh(auth_store, |token| {
            let value = params.clone();
            async move { payment_service::list_orders(value, &token).await }
        })
        .await
    });

    // Admin 订单
    let admin_orders = use_resource(move || async move {
        if !is_admin {
            return Ok(vec![]);
        }
        let status = status_filter();
        let params = if status != "all" {
            Some(AdminPaymentQueryParams::new().with_status(status.clone()))
        } else {
            None
        };
        with_auto_refresh(auth_store, |token| {
            let value = params.clone();
            async move {
                let client = get_client();
                AdminApi::new(&client)
                    .list_all_payment_orders(value.as_ref(), &token)
                    .await
            }
        })
        .await
    });

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "支付订单" }
            p { class: "page-description",
                if is_admin { "查看和管理平台所有支付订单" }
                else { "查看您的充値和支付记录" }
            }
        }

        // 状态筛选
        div { class: "toolbar",
            div { class: "toolbar-left",
                div { class: "filter-tabs",
                    for (val, label) in [("all", "全部"), ("pending", "待支付"), ("paid", "已支付"), ("failed", "已失败")] {
                        button {
                            class: if status_filter() == val { "filter-tab active" } else { "filter-tab" },
                            r#type: "button",
                            onclick: {
                                let val = val.to_string();
                                move |_| {
                                        *status_filter.write() = val.clone();
                                        page.set(1);
                                    }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }

        div { class: "card",
            if is_admin {
                {
                    let (is_empty, empty_text) = match admin_orders() {
                        None => (true, "加载中..."),
                        Some(Err(_)) => (true, "加载失败"),
                        Some(Ok(ref l)) if l.is_empty() => (true, "暂无订单记录"),
                        _ => (false, ""),
                    };
                    let admin_start = (page() as usize - 1) * PAGE_SIZE;
                    rsx! {
                        Table {
                            empty: is_empty,
                            empty_text: empty_text.to_string(),
                            col_count: 5,
                            thead {
                                tr {
                                    TableHead { "订单号" }
                                    TableHead { "用户" }
                                    TableHead { "金额" }
                                    TableHead { "状态" }
                                    TableHead { "创建时间" }
                                }
                            }
                            tbody {
                                if let Some(Ok(ref list)) = admin_orders() {
                                    for o in list.iter().skip(admin_start).take(PAGE_SIZE) {
                                        tr {
                                            td { code { "{o.out_trade_no}" } }
                                            td { "{o.user_id}" }
                                            td { "¥{o.amount:.2}" }
                                            td {
                                                Badge {
                                                    variant: status_to_variant(&o.status),
                                                    "{o.status}"
                                                }
                                            }
                                            td { { format_time(&o.created_at) } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                {
                    let (is_empty, empty_text) = match my_orders() {
                        None => (true, "加载中..."),
                        Some(Err(_)) => (true, "加载失败"),
                        Some(Ok(ref l)) if l.is_empty() => (true, "暂无订单记录"),
                        _ => (false, ""),
                    };
                    let my_start = (page() as usize - 1) * PAGE_SIZE;
                    rsx! {
                        Table {
                            empty: is_empty,
                            empty_text: empty_text.to_string(),
                            col_count: 5,
                            thead {
                                tr {
                                    TableHead { "订单号" }
                                    TableHead { "金额" }
                                    TableHead { "货币" }
                                    TableHead { "状态" }
                                    TableHead { "创建时间" }
                                }
                            }
                            tbody {
                                if let Some(Ok(ref list)) = my_orders() {
                                    for o in list.iter().skip(my_start).take(PAGE_SIZE) {
                                        tr {
                                            td { code { "{o.out_trade_no}" } }
                                            td { "¥{o.amount:.2}" }
                                            td { "{o.currency}" }
                                            td {
                                                Badge {
                                                    variant: status_to_variant(&o.status),
                                                    "{o.status}"
                                                }
                                            }
                                            td { { format_time(&o.created_at) } }
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
            let total = if is_admin {
                admin_orders().and_then(|r| r.ok()).map(|l| l.len()).unwrap_or(0)
            } else {
                my_orders().and_then(|r| r.ok()).map(|l| l.len()).unwrap_or(0)
            };
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
    }
}

fn status_to_variant(status: &str) -> BadgeVariant {
    match status {
        "paid" | "success" => BadgeVariant::Success,
        "pending" | "processing" => BadgeVariant::Warning,
        "failed" | "cancelled" => BadgeVariant::Error,
        _ => BadgeVariant::Neutral,
    }
}
