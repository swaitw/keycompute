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
/// 内含路由守卫：未登录时自动跳转到登录页
#[component]
pub fn AppLayout() -> Element {
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();
    let nav = use_navigator();

    // 路由守卫：未登录时重定向到登录页
    // 延迟执行以避免在渲染期间立即导航（Dioxus 禁止路由守卫范式）
    use_effect(move || {
        if !auth_store.is_authenticated() {
            nav.push(Route::Login {});
        }
    });

    // 认证状态未就绪时（没有 token）不渲染内容
    if !auth_store.is_authenticated() {
        return rsx! {};
    }

    let is_admin = user_store.is_admin();
    let user_name = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.display_name().to_string())
        .unwrap_or_default();

    let mut nav_sections = vec![
        NavSection {
            title: None,
            items: vec![
                NavItem::new("控制台", "/", NavIcon::Home),
                NavItem::new("API Key", "/api-keys", NavIcon::Key),
            ],
        },
        NavSection {
            title: Some("用量".to_string()),
            items: vec![
                NavItem::new("用量统计", "/usage", NavIcon::BarChart),
                NavItem::new("账单", "/billing", NavIcon::Wallet),
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
            user_name,
            current_path: use_route::<Route>().to_string(),
            Toast {}
            Outlet::<Route> {}
        }
    }
}
