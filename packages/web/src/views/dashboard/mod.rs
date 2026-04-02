use dioxus::prelude::*;
use ui::StatCard;

use crate::hooks::use_i18n::use_i18n;
use crate::router::Route;
use crate::services::api_client::with_auto_refresh;
use crate::services::usage_service;
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::UserStore;

#[component]
pub fn Dashboard() -> Element {
    let i18n = use_i18n();
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();

    // 获取用量统计
    let usage_stats = use_resource(move || {
        let auth = auth_store.clone();
        async move {
            with_auto_refresh(
                auth,
                |token| async move { usage_service::stats(&token).await },
            )
            .await
            .ok()
        }
    });

    let user_info = user_store.info.read();
    let greeting = if let Some(ref u) = *user_info {
        format!(
            "{}，{}",
            i18n.t("dashboard.greeting"),
            u.name.as_deref().unwrap_or(&u.email)
        )
    } else {
        i18n.t("dashboard.greeting").to_string()
    };
    drop(user_info);

    // 获取统计数据
    let stats = usage_stats.read();
    let (total_requests, total_cost) = if let Some(Some(ref s)) = *stats {
        (
            s.total_requests.to_string(),
            format!("¥{:.2}", s.total_cost),
        )
    } else {
        ("-".to_string(), "-".to_string())
    };
    drop(stats);

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "{greeting}" }
                p { class: "page-subtitle", {i18n.t("dashboard.subtitle")} }
            }

            div {
                class: "stats-grid",
                StatCard {
                    title: "API 调用".to_string(),
                    value: total_requests,
                    description: "本周总计".to_string(),
                }
                StatCard {
                    title: "余额".to_string(),
                    value: "¥0.00".to_string(),
                    description: "可用余额".to_string(),
                }
                StatCard {
                    title: "活跃 Key".to_string(),
                    value: "0".to_string(),
                    description: "总数".to_string(),
                }
                StatCard {
                    title: "本周费用".to_string(),
                    value: total_cost,
                    description: "已使用".to_string(),
                }
            }

            div {
                class: "section",
                h2 { class: "section-title", "快速链接" }
                div {
                    class: "quick-links",
                    QuickLink { route: Route::ApiKeyList {}, label: "管理 API Keys".to_string() }
                    QuickLink { route: Route::PaymentsOverview {}, label: "充值".to_string() }
                    QuickLink { route: Route::UserProfile {}, label: "账户设置".to_string() }
                }
            }
        }
    }
}

#[component]
fn QuickLink(route: Route, label: String) -> Element {
    let nav = use_navigator();
    rsx! {
        button {
            class: "quick-link-card",
            onclick: move |_| { nav.push(route.clone()); },
            "{label}"
        }
    }
}
