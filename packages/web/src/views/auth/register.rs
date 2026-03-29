use dioxus::prelude::*;

use crate::router::Route;
use crate::services::auth_service;

#[component]
pub fn Register() -> Element {
    let mut name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let nav = use_navigator();

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        if name().is_empty() || email().is_empty() || password().is_empty() {
            error_msg.set(Some("请填写所有必填项".to_string()));
            return;
        }
        if password() != confirm_password() {
            error_msg.set(Some("两次密码输入不一致".to_string()));
            return;
        }
        loading.set(true);
        error_msg.set(None);
        let name_val = name();
        let email_val = email();
        let password_val = password();
        spawn(async move {
            match auth_service::register(&email_val, &password_val, Some(name_val.as_str())).await {
                Ok(_) => {
                    nav.push(Route::Login {});
                }
                Err(e) => {
                    error_msg.set(Some(format!("注册失败：{e}")));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "auth-page",
            div {
                class: "auth-card",
                div {
                    class: "auth-header",
                    h1 { class: "auth-title", "注册" }
                    p { class: "auth-subtitle", "创建您的账户" }
                }

                if let Some(err) = error_msg() {
                    div { class: "alert alert-error", "{err}" }
                }

                form {
                    onsubmit: on_submit,
                    div {
                        class: "form-group",
                        label { class: "form-label", "姓名" }
                        input {
                            class: "form-input",
                            r#type: "text",
                            placeholder: "请输入姓名",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", "邮箱" }
                        input {
                            class: "form-input",
                            r#type: "email",
                            placeholder: "请输入邮箱",
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", "密码" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: "请输入密码（至少8位）",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", "确认密码" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: "再次输入密码",
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary btn-full",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() { "注册中..." } else { "注册" }
                    }
                }

                div {
                    class: "auth-footer",
                    "已有账户？"
                    a { class: "link", href: "/auth/login", " 立即登录" }
                }
            }
        }
    }
}
