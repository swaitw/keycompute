use client_api::{
    AdminApi,
    api::admin::{UpdateUserRequest, UserDetail, UserQueryParams},
};
use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Pagination, Table, TableHead};

use crate::services::api_client::{get_client, with_auto_refresh};
use crate::stores::auth_store::AuthStore;
use crate::stores::ui_store::UiStore;
use crate::stores::user_store::UserStore;
use crate::utils::time::format_time;

const PAGE_SIZE: usize = 20;

#[component]
pub fn Users() -> Element {
    let user_store = use_context::<UserStore>();
    let is_admin = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.is_admin())
        .unwrap_or(false);

    if is_admin {
        rsx! { AdminUsersView {} }
    } else {
        rsx! { UserSelfView {} }
    }
}

// ── Admin 视图 ────────────────────────────────────────────────────────

#[component]
fn AdminUsersView() -> Element {
    let auth_store = use_context::<AuthStore>();
    let mut ui_store = use_context::<UiStore>();
    let mut search = use_signal(String::new);
    let mut page = use_signal(|| 1u32);

    // 编辑弹窗状态
    let mut edit_user = use_signal(|| Option::<UserDetail>::None);
    let mut edit_name = use_signal(String::new);
    let mut edit_role = use_signal(String::new);
    let mut edit_saving = use_signal(|| false);

    // 删除确认状态
    let mut delete_user = use_signal(|| Option::<UserDetail>::None);
    let mut delete_saving = use_signal(|| false);

    let mut users = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            let client = get_client();
            let params = UserQueryParams::new().with_limit(200);
            AdminApi::new(&client)
                .list_all_users(Some(&params), &token)
                .await
        })
        .await
    });

    let filtered_users = move || -> Vec<UserDetail> {
        let q = search().to_lowercase();
        match users() {
            Some(Ok(ref list)) => list
                .iter()
                .filter(|u| {
                    q.is_empty()
                        || u.email.to_lowercase().contains(&q)
                        || u.name.as_deref().unwrap_or("").to_lowercase().contains(&q)
                })
                .cloned()
                .collect::<Vec<UserDetail>>(),
            _ => vec![],
        }
    };

    let total_pages = move || {
        let len = filtered_users().len();
        len.div_ceil(PAGE_SIZE).max(1) as u32
    };

    let paged_users = move || {
        let p = page() as usize;
        let all = filtered_users();
        let start = (p - 1) * PAGE_SIZE;
        all.into_iter()
            .skip(start)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>()
    };

    // 提交编辑
    let on_edit_save = move |_| {
        let Some(u) = edit_user() else { return };
        let name_val = edit_name();
        let role_val = edit_role();
        let id = u.id.clone();
        edit_saving.set(true);
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            let client = get_client();
            let req = UpdateUserRequest {
                name: if name_val.trim().is_empty() {
                    None
                } else {
                    Some(name_val)
                },
                role: if role_val.trim().is_empty() {
                    None
                } else {
                    Some(role_val)
                },
            };
            match AdminApi::new(&client).update_user(&id, &req, &token).await {
                Ok(_) => {
                    ui_store.show_success("用户信息已更新");
                    edit_user.set(None);
                    users.restart();
                }
                Err(e) => {
                    ui_store.show_error(format!("更新失败：{e}"));
                }
            }
            edit_saving.set(false);
        });
    };

    // 确认删除
    let on_delete_confirm = move |_| {
        let Some(u) = delete_user() else { return };
        let id = u.id.clone();
        delete_saving.set(true);
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            let client = get_client();
            match AdminApi::new(&client).delete_user(&id, &token).await {
                Ok(_) => {
                    ui_store.show_success("用户已删除");
                    delete_user.set(None);
                    users.restart();
                }
                Err(e) => {
                    ui_store.show_error(format!("删除失败：{e}"));
                }
            }
            delete_saving.set(false);
        });
    };

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "用户管理" }
            p { class: "page-description", "查看和管理平台所有注册用户" }
        }

        div { class: "toolbar",
            div { class: "toolbar-left",
                div { class: "input-wrapper",
                    input {
                        class: "input-field",
                        r#type: "search",
                        placeholder: "搜索邮箱或用户名...",
                        value: "{search}",
                        oninput: move |e| {
                            *search.write() = e.value();
                            page.set(1);
                        },
                    }
                }
            }
        }

        div { class: "card",
            {
                let (is_empty, empty_text) = match users() {
                    None => (true, "加载中..."),
                    Some(Err(_)) => (true, "加载失败"),
                    Some(Ok(_)) if filtered_users().is_empty() => (true, "暂无用户数据"),
                    _ => (false, ""),
                };
                rsx! {
                    Table {
                        empty: is_empty,
                        empty_text: empty_text.to_string(),
                        col_count: 5,
                        thead {
                            tr {
                                TableHead { "用户" }
                                TableHead { "角色" }
                                TableHead { "租户" }
                                TableHead { "注册时间" }
                                TableHead { "操作" }
                            }
                        }
                        tbody {
                            for u in paged_users().iter() {
                                tr {
                                    td {
                                        div { class: "user-cell",
                                            span { class: "user-name",
                                                { u.name.clone().unwrap_or_else(|| u.email.clone()) }
                                            }
                                            span { class: "user-email text-secondary", "{u.email}" }
                                        }
                                    }
                                    td {
                                        Badge { variant: BadgeVariant::Info, "{u.role}" }
                                    }
                                    td { "{u.tenant_id}" }
                                    td { { format_time(&u.created_at) } }
                                    td {
                                        div { class: "btn-group",
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Small,
                                                onclick: {
                                                    let uu = u.clone();
                                                    move |_| {
                                                        edit_name.set(uu.name.clone().unwrap_or_default());
                                                        edit_role.set(uu.role.clone());
                                                        edit_user.set(Some(uu.clone()));
                                                    }
                                                },
                                                "编辑"
                                            }
                                            Button {
                                                variant: ButtonVariant::Danger,
                                                size: ButtonSize::Small,
                                                onclick: {
                                                    let uu = u.clone();
                                                    move |_| delete_user.set(Some(uu.clone()))
                                                },
                                                "删除"
                                            }
                                        }
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
                "共 { filtered_users().len() } 条"
            }
            Pagination {
                current: page(),
                total_pages: total_pages(),
                on_page_change: move |p| page.set(p),
            }
        }

        // ── 编辑用户弹窗 ──────────────────────────────────────────
        if edit_user().is_some() {
            div { class: "modal-backdrop",
                onclick: move |_| edit_user.set(None),
                div {
                    class: "modal",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h2 { class: "modal-title", "编辑用户" }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| edit_user.set(None),
                            "✕"
                        }
                    }
                    div { class: "modal-body",
                        div { class: "form-group",
                            label { class: "form-label", "显示名称" }
                            input {
                                class: "input-field",
                                placeholder: "留空则不修改",
                                value: "{edit_name}",
                                oninput: move |e| *edit_name.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "角色" }
                            select {
                                class: "input-field",
                                value: "{edit_role}",
                                onchange: move |e| *edit_role.write() = e.value(),
                                option { value: "user", "user（普通用户）" }
                                option { value: "admin", "admin（管理员）" }
                            }
                        }
                    }
                    div { class: "modal-footer",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| edit_user.set(None),
                            "取消"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            loading: edit_saving(),
                            onclick: on_edit_save,
                            if edit_saving() { "保存中..." } else { "保存" }
                        }
                    }
                }
            }
        }

        // ── 删除确认弹窗 ──────────────────────────────────────────
        if let Some(ref du) = delete_user() {
            div { class: "modal-backdrop",
                onclick: move |_| delete_user.set(None),
                div {
                    class: "modal",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h2 { class: "modal-title", "确认删除" }
                    }
                    div { class: "modal-body",
                        p {
                            "确定要删除用户 "
                            strong { { du.name.clone().unwrap_or_else(|| du.email.clone()) } }
                            "（{du.email}）吗？此操作不可撤销。"
                        }
                    }
                    div { class: "modal-footer",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| delete_user.set(None),
                            "取消"
                        }
                        Button {
                            variant: ButtonVariant::Danger,
                            loading: delete_saving(),
                            onclick: on_delete_confirm,
                            if delete_saving() { "删除中..." } else { "确认删除" }
                        }
                    }
                }
            }
        }
    }
}

// ── 普通用户视图 ──────────────────────────────────────────────────────

#[component]
fn UserSelfView() -> Element {
    let user_store = use_context::<UserStore>();
    let user_info = user_store.info.read();

    let display_name = user_info
        .as_ref()
        .map(|u| u.display_name().to_string())
        .unwrap_or_default();
    let email = user_info
        .as_ref()
        .map(|u| u.email.clone())
        .unwrap_or_default();
    let role = user_info
        .as_ref()
        .map(|u| u.role.clone())
        .unwrap_or_default();

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "我的账户" }
            p { class: "page-description", "查看和管理您的个人账户信息" }
        }

        div { class: "card",
            div { class: "card-header",
                h3 { class: "card-title", "账户信息" }
                Button {
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Small,
                    onclick: move |_| {},
                    "编辑资料"
                }
            }
            div { class: "card-body",
                div { class: "info-grid",
                    div { class: "info-item",
                        span { class: "info-label", "显示名称" }
                        span { class: "info-value", "{display_name}" }
                    }
                    div { class: "info-item",
                        span { class: "info-label", "邮笱" }
                        span { class: "info-value", "{email}" }
                    }
                    div { class: "info-item",
                        span { class: "info-label", "角色" }
                        Badge { variant: BadgeVariant::Info, "{role}" }
                    }
                }
            }
        }
    }
}
