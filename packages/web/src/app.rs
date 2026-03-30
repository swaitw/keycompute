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
use ui::{AppShell, NavItem, NavSection, UserMenuAction};

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
/// 内含路由守卫：未登录时立即重定向到登录页，避免闪屏
#[component]
pub fn AppLayout() -> Element {
    let user_store = use_context::<UserStore>();
    let mut auth_store = use_context::<AuthStore>();
    let ui_store = use_context::<UiStore>();
    let nav = use_navigator();
    let mut user_store_write = use_context::<UserStore>();

    // 同步检查认证状态：在渲染之前立即判断，未登录则渲染重定向占位符
    // 同时通过 use_effect 执行实际导航（Dioxus 要求导航在 effect 中进行）
    let is_auth = auth_store.is_authenticated();

    use_effect(move || {
        if !auth_store.is_authenticated() {
            nav.replace(Route::Login {});
        }
    });

    // 未登录时渲染全屏加载态，use_effect 会在下一帧立即触发跳转
    // 避免将受保护页面内容闪现给未认证用户
    if !is_auth {
        return rsx! {
            div {
                class: "auth-redirect-loading",
                style: "display:flex;align-items:center;justify-content:center;height:100vh;background:var(--bg-primary,#f8fafc)",
                div {
                    style: "display:flex;flex-direction:column;align-items:center;gap:12px",
                    div {
                        class: "spinner",
                        style: "width:32px;height:32px",
                        role: "status",
                        "aria-label": "跳转中",
                    }
                    span {
                        style: "color:var(--text-secondary,#64748b);font-size:14px",
                        "正在跳转到登录页…"
                    }
                }
            }
        };
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
            items: vec![NavItem::new("用量统计", "/usage", NavIcon::BarChart)],
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
            on_user_menu: move |action: UserMenuAction| match action {
                UserMenuAction::Profile => { nav.push(Route::UserProfile {}); }
                UserMenuAction::Settings => { nav.push(Route::UserSettings {}); }
                UserMenuAction::Logout => {
                    auth_store.logout();
                    // 清空用户信息，避免登出后旧数据残留
                    *user_store_write.info.write() = None;
                    nav.replace(Route::Login {});
                }
            },
            Toast { toast: ui_store.toast }
            Outlet::<Route> {}
        }
    }
}

/// Admin 专属路由守卫层
///
/// 嵌套在 AppLayout 内部，仅允许 admin 角色访问 /admin/* 页面。
/// 非 admin 用户会被重定向到首页，同时显示无权提示。
#[component]
pub fn AdminLayout() -> Element {
    let user_store = use_context::<UserStore>();
    let mut ui_store = use_context::<UiStore>();
    let nav = use_navigator();

    let is_admin = user_store.is_admin();
    // 用户信息已加载（info 不为 None）时才做判断，避免初始化闪屏
    let info_loaded = user_store.info.read().is_some();

    use_effect(move || {
        if info_loaded && !user_store.is_admin() {
            ui_store.show_error("权限不足：该页面仅管理员可访问");
            nav.replace(Route::Dashboard {});
        }
    });

    // 用户信息尚未加载完成，显示等待占位符
    if !info_loaded {
        return rsx! {
            div {
                class: "admin-guard-loading",
                style: "display:flex;align-items:center;justify-content:center;padding:60px",
                div { class: "spinner", style: "width:24px;height:24px" }
            }
        };
    }

    // 已加载但不是 admin，显示空内容（effect 会立即跳转）
    if !is_admin {
        return rsx! {};
    }

    rsx! {
        Outlet::<Route> {}
    }
}
