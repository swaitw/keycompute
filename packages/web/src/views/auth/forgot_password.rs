use dioxus::prelude::*;

use crate::services::auth_service;

#[component]
pub fn ForgotPassword() -> Element {
    let mut email = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut sent = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        if email().is_empty() {
            error_msg.set(Some("请输入邮箱地址".to_string()));
            return;
        }
        loading.set(true);
        error_msg.set(None);
        let email_val = email();
        spawn(async move {
            match auth_service::forgot_password(&email_val).await {
                Ok(_) => {
                    sent.set(true);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("发送失败：{e}")));
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
                    h1 { class: "auth-title", "重置密码" }
                    p { class: "auth-subtitle", "输入您的邮箱，我们将发送重置链接" }
                }

                if sent() {
                    div {
                        class: "alert alert-success",
                        "重置链接已发送到您的邮箱，请查收"
                    }
                } else {
                    if let Some(err) = error_msg() {
                        div { class: "alert alert-error", "{err}" }
                    }
                    form {
                        onsubmit: on_submit,
                        div {
                            class: "form-group",
                            label { class: "form-label", "邮箱" }
                            input {
                                class: "form-input",
                                r#type: "email",
                                placeholder: "请输入注册邮箱",
                                value: "{email}",
                                oninput: move |e| email.set(e.value()),
                            }
                        }
                        button {
                            class: "btn btn-primary btn-full",
                            r#type: "submit",
                            disabled: loading(),
                            if loading() { "发送中..." } else { "发送重置链接" }
                        }
                    }
                }

                div {
                    class: "auth-footer",
                    a { class: "link", href: "/auth/login", "返回登录" }
                }
            }
        }
    }
}
