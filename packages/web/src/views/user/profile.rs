#![allow(clippy::clone_on_copy)]

use dioxus::prelude::*;

use crate::services::api_client::with_auto_refresh;
use crate::services::user_service;
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::{UserInfo, UserStore};

#[component]
pub fn UserProfile() -> Element {
    let auth_store = use_context::<AuthStore>();
    let mut user_store = use_context::<UserStore>();

    let mut edit_mode = use_signal(|| false);
    let mut edit_name = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut save_msg = use_signal(|| Option::<String>::None);
    let mut save_error = use_signal(|| Option::<String>::None);

    // 如果 UserStore 没有数据，主动获取
    let _user_data = use_resource(move || {
        let auth = auth_store.clone();
        async move {
            // 先检查是否已有数据
            if user_store.info.read().is_some() {
                return Ok(());
            }
            // 获取当前用户信息
            with_auto_refresh(auth, |token| async move {
                let user = user_service::get_current_user(&token).await?;
                *user_store.info.write() = Some(UserInfo {
                    id: user.id.to_string(),
                    email: user.email,
                    name: user.name,
                    role: user.role,
                    tenant_id: user.tenant_id.to_string(),
                });
                Ok::<(), client_api::ClientError>(())
            })
            .await
        }
    });

    // 从 UserStore 读取当前用户
    let user_info = user_store.info.read();
    let display_name = user_info
        .as_ref()
        .map(|u| u.name.as_deref().unwrap_or("-").to_string())
        .unwrap_or_default();
    let email = user_info
        .as_ref()
        .map(|u| u.email.clone())
        .unwrap_or_default();
    let role = user_info
        .as_ref()
        .map(|u| u.role.clone())
        .unwrap_or_default();
    let avatar = user_info.as_ref().map(|u| u.avatar_char()).unwrap_or('U');
    let has_user = user_info.is_some();
    drop(user_info);

    let on_edit_start = move |_| {
        let name = user_store
            .info
            .read()
            .as_ref()
            .and_then(|u| u.name.clone())
            .unwrap_or_default();
        edit_name.set(name);
        save_msg.set(None);
        save_error.set(None);
        edit_mode.set(true);
    };

    let on_save = move |evt: Event<FormData>| {
        evt.prevent_default();
        saving.set(true);
        save_error.set(None);
        let name_val = edit_name();
        let name_opt = if name_val.trim().is_empty() {
            None
        } else {
            Some(name_val)
        };
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            match user_service::update_profile(name_opt.clone(), &token).await {
                Ok(updated) => {
                    *user_store.info.write() = Some(UserInfo {
                        id: updated.id.to_string(),
                        email: updated.email,
                        name: updated.name,
                        role: updated.role,
                        tenant_id: updated.tenant_id.to_string(),
                    });
                    save_msg.set(Some("保存成功".to_string()));
                    edit_mode.set(false);
                    saving.set(false);
                }
                Err(e) => {
                    save_error.set(Some(format!("保存失败：{e}")));
                    saving.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "个人资料" }
            }

            if let Some(msg) = save_msg() {
                div { class: "alert alert-success", "{msg}" }
            }

            div {
                class: "card",
                if has_user {
                    div {
                        class: "profile-avatar",
                        span { class: "avatar-char", "{avatar}" }
                    }

                    if edit_mode() {
                        // 编辑模式
                        form {
                            onsubmit: on_save,
                            div {
                                class: "profile-info",
                                div {
                                    class: "form-group",
                                    label { class: "form-label", "姓名" }
                                    input {
                                        class: "form-input",
                                        r#type: "text",
                                        value: "{edit_name}",
                                        oninput: move |e| edit_name.set(e.value()),
                                    }
                                }
                                div {
                                    class: "form-group",
                                    label { class: "form-label", "邮箱" }
                                    p { class: "form-value text-muted", "{email}" }
                                }
                                div {
                                    class: "form-group",
                                    label { class: "form-label", "角色" }
                                    p { class: "form-value", "{role}" }
                                }
                                if let Some(err) = save_error() {
                                    div { class: "alert alert-error", "{err}" }
                                }
                                div {
                                    class: "form-actions",
                                    button {
                                        class: "btn btn-ghost",
                                        r#type: "button",
                                        onclick: move |_| edit_mode.set(false),
                                        "取消"
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        r#type: "submit",
                                        disabled: saving(),
                                        if saving() { "保存中..." } else { "保存" }
                                    }
                                }
                            }
                        }
                    } else {
                        // 展示模式
                        div {
                            class: "profile-info",
                            div {
                                class: "form-group",
                                label { class: "form-label", "姓名" }
                                p { class: "form-value", "{display_name}" }
                            }
                            div {
                                class: "form-group",
                                label { class: "form-label", "邮箱" }
                                p { class: "form-value", "{email}" }
                            }
                            div {
                                class: "form-group",
                                label { class: "form-label", "角色" }
                                p { class: "form-value", "{role}" }
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: on_edit_start,
                                "编辑资料"
                            }
                        }
                    }
                } else {
                    div { class: "empty-state", p { "加载中..." } }
                }
            }
        }
    }
}
