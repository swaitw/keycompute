use dioxus::prelude::*;

use crate::i18n::Lang;
use crate::router::Route;
use crate::services::user_service;
use crate::stores::{
    auth_store::AuthStore,
    ui_store::UiStore,
    user_store::{UserInfo, UserStore},
};
use crate::views::shared::Toast;
use ui::layout::sidebar::NavIcon;
use ui::{AppShell, NavItem, NavSection};

/// 根组件：提供所有全局 context，挂载路由
#[component]
pub fn App() -> Element {
    // 全局 context providers（必须在组件树顶层调用）
    let auth_store = use_context_provider(AuthStore::new);
    let mut user_store = use_context_provider(UserStore::new);
    let _ui_store = use_context_provider(UiStore::new);
    let _lang = use_context_provider(|| use_signal(Lang::default));

    // App 启动时，若 localStorage 已有 token，自动拉取用户信息
    use_effect(move || {
        if let Some(token) = auth_store.token() {
            spawn(async move {
                if let Ok(user) = user_service::get_current_user(&token).await {
                    *user_store.info.write() = Some(UserInfo {
                        id: user.id.to_string(),
                        email: user.email,
                        name: user.name,
                        role: user.role,
                        tenant_id: user.tenant_id.to_string(),
                    });
                }
            });
        }
    });

    rsx! {
        Router::<Route> {}
    }
}

/// 带 AppShell 侧边栏布局的页面外壳
#[component]
pub fn AppLayout() -> Element {
    let user_store = use_context::<UserStore>();
    let is_admin = user_store.is_admin();

    let mut nav_sections = vec![
        NavSection {
            title: None,
            items: vec![
                NavItem::new("控制台", "/", NavIcon::Home),
                NavItem::new("API Key", "/api-keys", NavIcon::Key),
            ],
        },
        NavSection {
            title: Some("账务".to_string()),
            items: vec![
                NavItem::new("支付与账单", "/payments", NavIcon::Wallet),
                NavItem::new("分发管理", "/distribution", NavIcon::Share),
            ],
        },
        NavSection {
            title: Some("账户".to_string()),
            items: vec![
                NavItem::new("个人资料", "/user/profile", NavIcon::User),
                NavItem::new("账户设置", "/user/settings", NavIcon::Settings),
            ],
        },
    ];

    // Admin 专属导航分组（仅 admin 角色可见）
    if is_admin {
        nav_sections.push(NavSection {
            title: Some("管理".to_string()),
            items: vec![
                NavItem::new("用户管理", "/admin/users", NavIcon::User),
                NavItem::new("渠道账号", "/admin/accounts", NavIcon::Key),
                NavItem::new("计费定价", "/admin/pricing", NavIcon::Wallet),
                NavItem::new("支付订单", "/admin/payment-orders", NavIcon::Wallet),
                NavItem::new("分销记录", "/admin/distribution-records", NavIcon::Share),
                NavItem::new("租户管理", "/admin/tenants", NavIcon::Home),
                NavItem::new("系统诊断", "/admin/system", NavIcon::Settings),
                NavItem::new("系统设置", "/admin/settings", NavIcon::Settings),
            ],
        });
    }

    rsx! {
        AppShell {
            nav_sections,
            Toast {}
            Outlet::<Route> {}
        }
    }
}
