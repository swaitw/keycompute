use dioxus::prelude::*;

use crate::hooks::use_i18n::use_i18n;
use crate::router::Route;
use crate::services::{payment_service, usage_service};
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::UserStore;

#[component]
pub fn Dashboard() -> Element {
    let i18n = use_i18n();
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();

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

    // 拉取用量统计
    let usage_stats = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        usage_service::stats(&token).await
    });

    // 拉取账户余额
    let balance = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        payment_service::get_balance(&token).await
    });

    // 拉取 API Key 数量（利用展示活跃 Key 数）
    let api_keys = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        crate::services::api_key_service::list(&token).await
    });

    let total_requests = match usage_stats() {
        Some(Ok(ref s)) => s.total_requests.to_string(),
        Some(Err(_)) => "加载失败".to_string(),
        None => "加载中...".to_string(),
    };
    let today_cost = match usage_stats() {
        Some(Ok(ref s)) => format!("¥{:.4}", s.total_cost),
        _ => "—".to_string(),
    };
    let balance_val = match balance() {
        Some(Ok(ref b)) => format!("¥{:.2}", b.balance),
        Some(Err(_)) => "加载失败".to_string(),
        None => "加载中...".to_string(),
    };
    let active_keys = match api_keys() {
        Some(Ok(ref keys)) => keys.iter().filter(|k| !k.revoked).count().to_string(),
        Some(Err(_)) => "—".to_string(),
        None => "—".to_string(),
    };

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
                    title: i18n.t("dashboard.api_calls").to_string(),
                    value: "{total_requests}",
                    label: i18n.t("dashboard.weekly_total").to_string(),
                    icon: "key",
                }
                StatCard {
                    title: i18n.t("dashboard.balance").to_string(),
                    value: "{balance_val}",
                    label: i18n.t("dashboard.available").to_string(),
                    icon: "wallet",
                }
                StatCard {
                    title: i18n.t("dashboard.active_keys").to_string(),
                    value: "{active_keys}",
                    label: i18n.t("dashboard.total").to_string(),
                    icon: "list",
                }
                StatCard {
                    title: i18n.t("dashboard.weekly_cost").to_string(),
                    value: "{today_cost}",
                    label: i18n.t("dashboard.used").to_string(),
                    icon: "chart",
                }
            }

            div {
                class: "section",
                h2 { class: "section-title", {i18n.t("dashboard.quick_links")} }
                div {
                    class: "quick-links",
                    QuickLink { route: Route::ApiKeyList {}, label: i18n.t("dashboard.manage_api_keys").to_string() }
                    QuickLink { route: Route::PaymentsOverview {}, label: i18n.t("dashboard.recharge").to_string() }
                    QuickLink { route: Route::UserProfile {}, label: i18n.t("dashboard.account_settings").to_string() }
                }
            }
        }
    }
}

#[component]
fn StatCard(title: String, value: String, label: String, icon: String) -> Element {
    rsx! {
        div {
            class: "stat-card",
            div {
                class: "stat-icon stat-icon-{icon}",
            }
            div {
                class: "stat-body",
                p { class: "stat-title", "{title}" }
                p { class: "stat-value", "{value}" }
                p { class: "stat-label", "{label}" }
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
