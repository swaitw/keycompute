use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Table, TableHead};

use crate::services::distribution_service;
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::UserStore;

/// 分销记录页面
///
/// - 普通用户：仅查看自己的分销收益记录
/// - Admin：查看全平台分销记录，配置分销规则
#[component]
pub fn DistributionRecords() -> Element {
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();
    let is_admin = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.is_admin())
        .unwrap_or(false);

    // 收益数据（普通用户）
    let earnings = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        distribution_service::get_earnings(&token).await
    });

    // Admin 分销记录
    let admin_records = use_resource(move || async move {
        if !is_admin {
            return Ok(vec![]);
        }
        let token = auth_store.token().unwrap_or_default();
        use crate::services::api_client::get_client;
        use client_api::DistributionApi;
        let client = get_client();
        DistributionApi::new(&client)
            .list_distribution_records(None, &token)
            .await
    });

    let total_earnings = match earnings() {
        Some(Ok(ref e)) => format!("¥{:.2}", e.total_earnings),
        _ => "¥ 0.00".to_string(),
    };
    let available = match earnings() {
        Some(Ok(ref e)) => format!("¥{:.2}", e.available_earnings),
        _ => "¥ 0.00".to_string(),
    };
    let pending = match earnings() {
        Some(Ok(ref e)) => format!("¥{:.2}", e.pending_earnings),
        _ => "¥ 0.00".to_string(),
    };

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "分销记录" }
            p { class: "page-description",
                if is_admin { "查看全平台分销收益记录，管理分销规则" }
                else { "查看您通过邀请获得的分销收益明细" }
            }
        }

        // 收益统计卡片
        div { class: "stats-grid",
            div { class: "stat-card card",
                div { class: "card-body",
                    p { class: "stat-label", "总收益" }
                    p { class: "stat-value", "{total_earnings}" }
                }
            }
            div { class: "stat-card card",
                div { class: "card-body",
                    p { class: "stat-label", "可用余额" }
                    p { class: "stat-value", "{available}" }
                }
            }
            div { class: "stat-card card",
                div { class: "card-body",
                    p { class: "stat-label", "待结算" }
                    p { class: "stat-value", "{pending}" }
                }
            }
        }

        if is_admin {
            // Admin 工具栏：规则配置入口
            div { class: "toolbar",
                div { class: "toolbar-right",
                    button { class: "btn btn-secondary btn-sm", r#type: "button",
                        "分销规则配置"
                    }
                }
            }
        }

        {
            let (is_empty, empty_text, col) = if is_admin {
                match admin_records() {
                    None => (true, "加载中...", 7u32),
                    Some(Err(_)) => (true, "加载失败", 7),
                    Some(Ok(ref l)) if l.is_empty() => (true, "暂无分销记录", 7),
                    _ => (false, "", 7),
                }
            } else {
                (true, "暂无分销记录", 6u32)
            };
            rsx! {
                Table {
                    empty: is_empty,
                    empty_text: empty_text.to_string(),
                    col_count: col,
                    thead {
                        tr {
                            TableHead { "记录编号" }
                            TableHead { "来源用户" }
                            TableHead { "消费金额" }
                            TableHead { "分销金额" }
                            TableHead { "状态" }
                            TableHead { "创建时间" }
                            if is_admin {
                                TableHead { "推荐人" }
                            }
                        }
                    }
                    tbody {
                        if is_admin {
                            if let Some(Ok(ref list)) = admin_records() {
                                for rec in list.iter() {
                                    tr {
                                        td { code { "{rec.id}" } }
                                        td { "{rec.referred_id}" }
                                        td { "¥{rec.amount:.2}" }
                                        td { "¥{rec.commission:.2}" }
                                        td {
                                            Badge {
                                                variant: dist_status_variant(&rec.status),
                                                "{rec.status}"
                                            }
                                        }
                                        td { "{rec.created_at}" }
                                        td { "{rec.referrer_id}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        div { class: "pagination",
            span { class: "pagination-info",
                {
                    let count = if is_admin {
                        admin_records().and_then(|r| r.ok()).map(|l| l.len()).unwrap_or(0)
                    } else {
                        // 普通用户从收益统计中取推荐人数
                        earnings()
                            .and_then(|r| r.ok())
                            .map(|e| e.referral_count.max(0) as usize)
                            .unwrap_or(0)
                    };
                    format!("共 {} 条", count)
                }
            }
        }
    }
}

fn dist_status_variant(status: &str) -> BadgeVariant {
    match status {
        "settled" | "paid" => BadgeVariant::Success,
        "pending" => BadgeVariant::Warning,
        "cancelled" | "failed" => BadgeVariant::Error,
        _ => BadgeVariant::Neutral,
    }
}
