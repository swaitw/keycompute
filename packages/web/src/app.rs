use dioxus::prelude::*;

use crate::i18n::Lang;
use crate::router::Route;
use crate::services::api_client::get_client;
use crate::services::user_service;
use crate::stores::{
    auth_store::AuthStore,
    ui_store::{ToastMsg, UiStore},
    user_store::{UserInfo, UserStore},
};
use crate::views::shared::Toast;
use ui::layout::sidebar::NavIcon;
use ui::{AppShell, NavItem, NavSection, UserMenuAction};

/// 根组件：提供所有全局 context，挂载路由
#[component]
pub fn App() -> Element {
    // 所有 Signal 必须在组件顶层直接创建，不能在 hook 的闭包里调用 use_signal
    let auth_initial = AuthStore::load_from_storage();
    let auth_state = use_signal(|| auth_initial);
    let user_info = use_signal(|| None::<UserInfo>);
    let toast_signal = use_signal(|| None::<ToastMsg>);
    let lang_signal = use_signal(Lang::default);

    let auth_store = use_context_provider(|| AuthStore::new(auth_state));
    let mut user_store = use_context_provider(|| UserStore::new(user_info));
    let _ui_store = use_context_provider(|| UiStore::new(toast_signal));
    let _lang = use_context_provider(|| lang_signal);

    // App 启动时或登录状态变化时，若已有 token，自动拉取用户信息
    use_effect(move || {
        // 依赖 auth_store 的认证状态，登录/登出时会重新执行
        let is_auth = auth_store.is_authenticated();
        if is_auth && let Some(token) = auth_store.token() {
            // 恢复 token 到 API 客户端
            get_client().set_token(&token);
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

    // 路径由 Route 枚举派生，避免硬编码字符串拼写错误
    let r_dashboard = Route::Dashboard {}.to_string();
    let r_api_keys = Route::ApiKeyList {}.to_string();
    let r_usage = Route::Usage {}.to_string();
    let r_payments = Route::PaymentsOverview {}.to_string();
    let r_distribution = Route::DistributionOverview {}.to_string();
    let r_profile = Route::UserProfile {}.to_string();
    let r_settings = Route::UserSettings {}.to_string();
    let r_admin_users = Route::Users {}.to_string();
    let r_admin_accounts = Route::Accounts {}.to_string();
    let r_admin_pricing = Route::Pricing {}.to_string();
    let r_admin_payment_orders = Route::PaymentOrders {}.to_string();
    let r_admin_distribution = Route::DistributionRecords {}.to_string();
    let r_admin_tenants = Route::Tenants {}.to_string();
    let r_admin_system = Route::System {}.to_string();
    let r_admin_system_settings = Route::Settings {}.to_string();

    let mut nav_sections = vec![
        NavSection {
            title: None,
            items: vec![
                NavItem::new("控制台", r_dashboard, NavIcon::Home),
                NavItem::new("API Key", r_api_keys, NavIcon::Key),
            ],
        },
        NavSection {
            title: Some("用量".to_string()),
            items: vec![NavItem::new("用量统计", r_usage, NavIcon::BarChart)],
        },
        NavSection {
            title: Some("账务".to_string()),
            items: vec![
                NavItem::new("支付与账单", r_payments, NavIcon::Wallet),
                NavItem::new("分发管理", r_distribution, NavIcon::Share),
            ],
        },
        NavSection {
            title: Some("账户".to_string()),
            items: vec![
                NavItem::new("个人资料", r_profile, NavIcon::User),
                NavItem::new("账户设置", r_settings, NavIcon::Settings),
            ],
        },
    ];

    // Admin 专属导航分组（仅 admin 角色可见）
    if is_admin {
        nav_sections.push(NavSection {
            title: Some("管理".to_string()),
            items: vec![
                NavItem::new("用户管理", r_admin_users, NavIcon::User).admin(),
                NavItem::new("渠道账号", r_admin_accounts, NavIcon::Key).admin(),
                NavItem::new("计费定价", r_admin_pricing, NavIcon::Wallet).admin(),
                NavItem::new("支付订单", r_admin_payment_orders, NavIcon::Wallet).admin(),
                NavItem::new("分销记录", r_admin_distribution, NavIcon::Share).admin(),
                NavItem::new("租户管理", r_admin_tenants, NavIcon::Home).admin(),
                NavItem::new("系统诊断", r_admin_system, NavIcon::Settings).admin(),
                NavItem::new("系统设置", r_admin_system_settings, NavIcon::Settings).admin(),
            ],
        });
    }

    let current_path = use_route::<Route>().to_string();

    rsx! {
        AppShell {
            nav_sections,
            user_name,
            current_path,
            on_user_menu: move |action: UserMenuAction| match action {
                UserMenuAction::Profile => { nav.push(Route::UserProfile {}); }
                UserMenuAction::Settings => { nav.push(Route::UserSettings {}); }
                UserMenuAction::Logout => {
                    auth_store.logout();
                    // 清除 API 客户端 token
                    get_client().clear_token();
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
