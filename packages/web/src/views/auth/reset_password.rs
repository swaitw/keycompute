use dioxus::prelude::*;

use crate::router::Route;
use crate::services::auth_service;

/// 重置密码页面
/// 路由：/auth/reset-password/:token
#[component]
pub fn ResetPassword(token: String) -> Element {
    let nav = use_navigator();

    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);

    let on_submit = {
        let token = token.clone();
        move |evt: Event<FormData>| {
            evt.prevent_default();
            let pwd = password();
            let cfm = confirm();

            if pwd.is_empty() || cfm.is_empty() {
                error_msg.set(Some("请填写所有字段".to_string()));
                return;
            }
            if pwd != cfm {
                error_msg.set(Some("两次输入的密码不一致".to_string()));
                return;
            }
            if pwd.len() < 8 {
                error_msg.set(Some("密码长度至少 8 位".to_string()));
                return;
            }

            let token = token.clone();
            submitting.set(true);
            error_msg.set(None);

            spawn(async move {
                match auth_service::reset_password(&token, &pwd).await {
                    Ok(_) => {
                        success.set(true);
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("重置失败：{e}")));
                    }
                }
                submitting.set(false);
            });
        }
    };

    rsx! {
        div {
            class: "auth-page",
            div {
                class: "auth-card",
                h1 { class: "auth-title", "重置密码" }

                if success() {
                    div { class: "alert alert-success",
                        p { "密码已重置成功！" }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| { nav.push(Route::Login {}); },
                            "前往登录"
                        }
                    }
                } else {
                    if let Some(msg) = error_msg() {
                        div { class: "alert alert-error", "{msg}" }
                    }

                    form {
                        onsubmit: on_submit,
                        div { class: "form-group",
                            label { class: "form-label", "新密码" }
                            input {
                                class: "form-input",
                                r#type: "password",
                                placeholder: "请输入新密码（至少 8 位）",
                                value: "{password}",
                                oninput: move |e| password.set(e.value()),
                                disabled: submitting(),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "确认密码" }
                            input {
                                class: "form-input",
                                r#type: "password",
                                placeholder: "请再次输入新密码",
                                value: "{confirm}",
                                oninput: move |e| confirm.set(e.value()),
                                disabled: submitting(),
                            }
                        }
                        button {
                            class: "btn btn-primary btn-full",
                            r#type: "submit",
                            disabled: submitting(),
                            if submitting() { "提交中..." } else { "确认重置" }
                        }
                    }

                    div { class: "auth-footer",
                        button {
                            class: "link-btn",
                            onclick: move |_| { nav.push(Route::Login {}); },
                            "返回登录"
                        }
                    }
                }
            }
        }
    }
}
