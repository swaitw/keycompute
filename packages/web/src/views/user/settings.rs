use dioxus::prelude::*;

use crate::services::user_service;
use crate::stores::auth_store::AuthStore;

#[component]
pub fn UserSettings() -> Element {
    let auth_store = use_context::<AuthStore>();
    let mut current_pwd = use_signal(String::new);
    let mut new_pwd = use_signal(String::new);
    let mut confirm_pwd = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut success_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        if current_pwd().is_empty() || new_pwd().is_empty() || confirm_pwd().is_empty() {
            error_msg.set(Some("请填写所有密码字段".to_string()));
            return;
        }
        if new_pwd() != confirm_pwd() {
            error_msg.set(Some("两次新密码输入不一致".to_string()));
            return;
        }
        if new_pwd().len() < 8 {
            error_msg.set(Some("新密码至少需要 8 位".to_string()));
            return;
        }
        saving.set(true);
        error_msg.set(None);
        success_msg.set(None);
        let cur = current_pwd();
        let nw = new_pwd();
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            match user_service::change_password(&cur, &nw, &token).await {
                Ok(_) => {
                    success_msg.set(Some("密码修改成功".to_string()));
                    current_pwd.set(String::new());
                    new_pwd.set(String::new());
                    confirm_pwd.set(String::new());
                    saving.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("修改失败：{e}")));
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
                h1 { class: "page-title", "账户设置" }
            }
            div {
                class: "card",
                h2 { class: "section-title", "修改密码" }

                if let Some(msg) = success_msg() {
                    div { class: "alert alert-success", "{msg}" }
                }
                if let Some(err) = error_msg() {
                    div { class: "alert alert-error", "{err}" }
                }

                form {
                    onsubmit: on_submit,
                    div {
                        class: "form-group",
                        label { class: "form-label", "当前密码" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: "请输入当前密码",
                            value: "{current_pwd}",
                            oninput: move |e| current_pwd.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", "新密码" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: "请输入新密码（至少8位）",
                            value: "{new_pwd}",
                            oninput: move |e| new_pwd.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", "确认新密码" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: "再次输入新密码",
                            value: "{confirm_pwd}",
                            oninput: move |e| confirm_pwd.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "submit",
                        disabled: saving(),
                        if saving() { "保存中..." } else { "保存修改" }
                    }
                }
            }
        }
    }
}
