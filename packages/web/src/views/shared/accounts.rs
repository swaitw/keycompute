use dioxus::prelude::*;
use ui::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Pagination, Table, TableHead};

const PAGE_SIZE: usize = 20;

use crate::services::{account_service, api_client::with_auto_refresh};
use crate::stores::auth_store::AuthStore;
use crate::stores::ui_store::UiStore;
use crate::stores::user_store::UserStore;
use crate::utils::time::format_time;

/// 账号管理页面（LLM 渠道配置）
///
/// - 普通用户：无权限提示
/// - Admin：管理 LLM Provider 渠道，支持测试连接、刷新状态
#[component]
pub fn Accounts() -> Element {
    let user_store = use_context::<UserStore>();
    let is_admin = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.is_admin())
        .unwrap_or(false);

    if is_admin {
        rsx! { AdminAccountsView {} }
    } else {
        rsx! { NoPermissionView { resource: "账号管理" } }
    }
}

// ── Admin 视图 ────────────────────────────────────────────────────────

#[component]
fn AdminAccountsView() -> Element {
    let auth_store = use_context::<AuthStore>();
    let mut ui_store = use_context::<UiStore>();
    let mut show_create = use_signal(|| false);
    let mut create_name = use_signal(String::new);
    let mut create_provider = use_signal(String::new);
    let mut create_api_key = use_signal(String::new);
    let mut create_api_base = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut error_msg = use_signal(String::new);
    let mut page = use_signal(|| 1u32);

    // 编辑弹窗状态
    let mut edit_id = use_signal(String::new);
    let mut edit_name = use_signal(String::new);
    let mut edit_api_key = use_signal(String::new);
    let mut edit_api_base = use_signal(String::new);
    let mut edit_is_active = use_signal(|| true);
    let mut show_edit = use_signal(|| false);
    let mut edit_saving = use_signal(|| false);
    let mut edit_error = use_signal(String::new);

    let mut accounts = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            account_service::list(None, &token).await
        })
        .await
    });

    let on_submit = move |_| {
        let name = create_name();
        let provider = create_provider();
        let api_key_val = create_api_key();
        let api_base = create_api_base();
        if name.is_empty() || provider.is_empty() || api_key_val.is_empty() {
            *error_msg.write() = "请填写必填项".to_string();
            return;
        }
        let token = auth_store.token().unwrap_or_default();
        *saving.write() = true;
        *error_msg.write() = String::new();
        spawn(async move {
            use client_api::api::admin::CreateAccountRequest;
            let mut req = CreateAccountRequest::new(name, provider, api_key_val);
            if !api_base.is_empty() {
                req = req.with_api_base(api_base);
            }
            match account_service::create(req, &token).await {
                Ok(_) => {
                    *show_create.write() = false;
                    create_name.write().clear();
                    create_provider.write().clear();
                    create_api_key.write().clear();
                    create_api_base.write().clear();
                    page.set(1);
                    accounts.restart();
                    ui_store.show_success("渠道已创建");
                }
                Err(e) => {
                    *error_msg.write() = format!("创建失败：{}", e);
                }
            }
            *saving.write() = false;
        });
    };

    // 提交编辑
    let on_edit_save = move |_| {
        let id = edit_id();
        let name_val = edit_name();
        let key_val = edit_api_key();
        let base_val = edit_api_base();
        let active = edit_is_active();
        if name_val.trim().is_empty() {
            *edit_error.write() = "渠道名称不能为空".to_string();
            return;
        }
        let token = auth_store.token().unwrap_or_default();
        edit_saving.set(true);
        *edit_error.write() = String::new();
        spawn(async move {
            use client_api::api::admin::UpdateAccountRequest;
            let mut req = UpdateAccountRequest::new()
                .with_name(name_val)
                .with_is_active(active);
            if !key_val.trim().is_empty() {
                req = req.with_api_key(key_val);
            }
            if !base_val.trim().is_empty() {
                req.api_base = Some(base_val);
            }
            match account_service::update(&id, req, &token).await {
                Ok(_) => {
                    show_edit.set(false);
                    accounts.restart();
                    ui_store.show_success("渠道已更新");
                }
                Err(e) => {
                    *edit_error.write() = format!("更新失败：{}", e);
                }
            }
            edit_saving.set(false);
        });
    };

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "账号管理" }
            p { class: "page-description", "管理 LLM Provider 渠道，配置 API Key 和模型映射" }
        }

        div { class: "toolbar",
            div { class: "toolbar-right",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Small,
                    onclick: move |_| *show_create.write() = true,
                    "+ 新增渠道"
                }
            }
        }

        {
            let (is_empty, empty_text) = match accounts() {
                None => (true, "加载中..."),
                Some(Err(_)) => (true, "加载失败"),
                Some(Ok(ref l)) if l.is_empty() => (true, "暂无渠道配置，请点击「新增渠道」添加"),
                _ => (false, ""),
            };
            let total = accounts().and_then(|r| r.ok()).map(|l| l.len()).unwrap_or(0);
            let total_pages = total.div_ceil(PAGE_SIZE).max(1) as u32;
            let start = (page() as usize - 1) * PAGE_SIZE;
            let paged_list: Vec<_> = accounts()
                .and_then(|r| r.ok())
                .map(|l| l.into_iter().skip(start).take(PAGE_SIZE).collect())
                .unwrap_or_default();
            rsx! {
                Table {
                    empty: is_empty,
                    empty_text: empty_text.to_string(),
                    col_count: 5,
                    thead {
                        tr {
                            TableHead { "渠道名称" }
                            TableHead { "Provider" }
                            TableHead { "状态" }
                            TableHead { "创建时间" }
                            TableHead { "操作" }
                        }
                    }
                    tbody {
                        if accounts().and_then(|r| r.ok()).is_some() {
                            for acc in paged_list.iter() {
                                tr {
                                    td { "{acc.name}" }
                                    td { "{acc.provider}" }
                                    td {
                                        if acc.is_active {
                                            Badge { variant: BadgeVariant::Success, "已启用" }
                                        } else {
                                            Badge { variant: BadgeVariant::Neutral, "已禁用" }
                                        }
                                    }
                                    td { { format_time(&acc.created_at) } }
                                    td {
                                        div { class: "btn-group",
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Small,
                                                onclick: {
                                                    let id = acc.id.clone();
                                                    let name = acc.name.clone();
                                                    let active = acc.is_active;
                                                    move |_| {
                                                        edit_id.set(id.clone());
                                                        edit_name.set(name.clone());
                                                        edit_api_key.set(String::new());
                                                        edit_api_base.set(String::new());
                                                        edit_is_active.set(active);
                                                        *edit_error.write() = String::new();
                                                        show_edit.set(true);
                                                    }
                                                },
                                                "编辑"
                                            }
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Small,
                                                onclick: {
                                                    let id = acc.id.clone();
                                                    move |_| {
                                                        let token = auth_store.token().unwrap_or_default();
                                                        let id = id.clone();
                                                        spawn(async move {
                                                            let _ = account_service::test(&id, &token).await;
                                                            accounts.restart();
                                                        });
                                                    }
                                                },
                                                "测试"
                                            }
                                            Button {
                                                variant: ButtonVariant::Danger,
                                                size: ButtonSize::Small,
                                                onclick: {
                                                    let id = acc.id.clone();
                                                    move |_| {
                                                        let token = auth_store.token().unwrap_or_default();
                                                        let id = id.clone();
                                                        spawn(async move {
                                                            let _ = account_service::delete(&id, &token).await;
                                                            accounts.restart();
                                                        });
                                                    }
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

        // 新增渠道弹窗
        if show_create() {
            div { class: "modal-backdrop",
                onclick: move |_| *show_create.write() = false,
                div {
                    class: "modal",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h2 { class: "modal-title", "新增 LLM 渠道" }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| *show_create.write() = false,
                            "✕"
                        }
                    }
                    div { class: "modal-body",
                        if !error_msg().is_empty() {
                            div { class: "alert alert-error",
                                span { "{error_msg}" }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "渠道名称 *" }
                            input {
                                class: "input-field",
                                placeholder: "如 OpenAI 官方",
                                value: "{create_name}",
                                oninput: move |e| *create_name.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Provider *" }
                            input {
                                class: "input-field",
                                placeholder: "如 openai、azure、anthropic",
                                value: "{create_provider}",
                                oninput: move |e| *create_provider.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "API Key *" }
                            input {
                                class: "input-field",
                                r#type: "password",
                                placeholder: "sk-...",
                                value: "{create_api_key}",
                                oninput: move |e| *create_api_key.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "自定义 Base URL（可选）" }
                            input {
                                class: "input-field",
                                placeholder: "https://api.openai.com/v1",
                                value: "{create_api_base}",
                                oninput: move |e| *create_api_base.write() = e.value(),
                            }
                        }
                    }
                    div { class: "modal-footer",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| *show_create.write() = false,
                            "取消"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            loading: saving(),
                            onclick: on_submit,
                            if saving() { "保存中..." } else { "保存" }
                        }
                    }
                }
            }
        }
        // 编辑渠道弹窗
        if show_edit() {
            div { class: "modal-backdrop",
                onclick: move |_| show_edit.set(false),
                div {
                    class: "modal",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h2 { class: "modal-title", "编辑 LLM 渠道" }
                        button {
                            class: "btn btn-ghost btn-sm",
                            r#type: "button",
                            onclick: move |_| show_edit.set(false),
                            "✕"
                        }
                    }
                    div { class: "modal-body",
                        if !edit_error().is_empty() {
                            div { class: "alert alert-error",
                                span { "{edit_error}" }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "渠道名称 *" }
                            input {
                                class: "input-field",
                                value: "{edit_name}",
                                oninput: move |e| *edit_name.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "新 API Key（留空则不修改）" }
                            input {
                                class: "input-field",
                                r#type: "password",
                                placeholder: "留空不修改当前 Key",
                                value: "{edit_api_key}",
                                oninput: move |e| *edit_api_key.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "自定义 Base URL（留空则不修改）" }
                            input {
                                class: "input-field",
                                placeholder: "https://api.openai.com/v1",
                                value: "{edit_api_base}",
                                oninput: move |e| *edit_api_base.write() = e.value(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label",
                                input {
                                    r#type: "checkbox",
                                    checked: edit_is_active(),
                                    oninput: move |e| edit_is_active.set(e.value() == "true"),
                                    style: "margin-right:6px",
                                }
                                "启用渠道"
                            }
                        }
                    }
                    div { class: "modal-footer",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| show_edit.set(false),
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
    }
}

// ── 无权限视图（共用组件）──────────────────────────────────────────────

#[component]
pub fn NoPermissionView(resource: String) -> Element {
    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "{resource}" }
        }
        div { class: "empty-state",
            div { class: "empty-icon", "🔒" }
            h3 { class: "empty-title", "暂无访问权限" }
            p { class: "empty-description",
                "您没有访问「{resource}」的权限，请联系管理员"
            }
        }
    }
}
