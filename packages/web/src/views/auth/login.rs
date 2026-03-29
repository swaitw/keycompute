use dioxus::prelude::*;

use crate::hooks::use_i18n::use_i18n;
use crate::router::Route;
use crate::services::auth_service;
use crate::services::user_service;
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::{UserInfo, UserStore};

#[component]
pub fn Login() -> Element {
    let i18n = use_i18n();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut auth_store = use_context::<AuthStore>();
    let mut user_store = use_context::<UserStore>();
    let nav = use_navigator();

    // 提前提取 &'static str，闭包只捕获 Copy 类型避免成为 FnOnce
    let t_fill_all = i18n.t("auth.fill_all");
    let t_login_failed = i18n.t("auth.login_failed");

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        let email_val = email();
        let password_val = password();
        if email_val.is_empty() || password_val.is_empty() {
            error_msg.set(Some(t_fill_all.to_string()));
            return;
        }
        loading.set(true);
        error_msg.set(None);
        spawn(async move {
            match auth_service::login(&email_val, &password_val).await {
                Ok(resp) => {
                    auth_store.login(resp.access_token.clone(), resp.refresh_token.clone());
                    // 拉取当前用户信息
                    if let Ok(user) = user_service::get_current_user(&resp.access_token).await {
                        *user_store.info.write() = Some(UserInfo {
                            id: user.id.to_string(),
                            email: user.email,
                            name: user.name,
                            role: user.role,
                            tenant_id: user.tenant_id.to_string(),
                        });
                    }
                    nav.push(Route::Dashboard {});
                }
                Err(e) => {
                    error_msg.set(Some(format!("{t_login_failed}：{e}")));
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
                    h1 { class: "auth-title", {i18n.t("auth.login")} }
                    p { class: "auth-subtitle", {i18n.t("auth.login_subtitle")} }
                }

                if let Some(err) = error_msg() {
                    div {
                        class: "alert alert-error",
                        "{err}"
                    }
                }

                form {
                    onsubmit: on_submit,
                    div {
                        class: "form-group",
                        label { class: "form-label", {i18n.t("auth.email")} }
                        input {
                            class: "form-input",
                            r#type: "email",
                            placeholder: i18n.t("auth.email_placeholder"),
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", {i18n.t("auth.password")} }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: i18n.t("auth.password_placeholder"),
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    div {
                        class: "form-actions",
                        button {
                            class: "link",
                            r#type: "button",
                            onclick: move |_| { nav.push(Route::ForgotPassword {}); },
                            {i18n.t("auth.forgot_password")}
                        }
                    }
                    button {
                        class: "btn btn-primary btn-full",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() { {i18n.t("auth.logging_in")} } else { {i18n.t("auth.login")} }
                    }
                }

                div {
                    class: "auth-footer",
                    {i18n.t("auth.no_account")}
                    button {
                        class: "link",
                        r#type: "button",
                        onclick: move |_| { nav.push(Route::Register {}); },
                        " ",
                        {i18n.t("auth.register_now")}
                    }
                }
            }
        }
    }
}
