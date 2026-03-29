use dioxus::prelude::*;

use crate::hooks::use_i18n::use_i18n;
use crate::router::Route;
use crate::services::auth_service;

#[component]
pub fn Register() -> Element {
    let i18n = use_i18n();
    let mut name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let nav = use_navigator();

    // 提前提取 &'static str，避免闭包成为 FnOnce
    let t_fill_required = i18n.t("auth.fill_required");
    let t_pwd_mismatch = i18n.t("form.password_mismatch");
    let t_register_failed = i18n.t("auth.register_failed");

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        if name().is_empty() || email().is_empty() || password().is_empty() {
            error_msg.set(Some(t_fill_required.to_string()));
            return;
        }
        if password() != confirm_password() {
            error_msg.set(Some(t_pwd_mismatch.to_string()));
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
                    error_msg.set(Some(format!("{t_register_failed}：{e}")));
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
                    h1 { class: "auth-title", {i18n.t("auth.register")} }
                    p { class: "auth-subtitle", {i18n.t("auth.register_subtitle")} }
                }

                if let Some(err) = error_msg() {
                    div { class: "alert alert-error", "{err}" }
                }

                form {
                    onsubmit: on_submit,
                    div {
                        class: "form-group",
                        label { class: "form-label", {i18n.t("auth.name")} }
                        input {
                            class: "form-input",
                            r#type: "text",
                            placeholder: i18n.t("auth.name_placeholder"),
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
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
                            placeholder: i18n.t("auth.password_min8"),
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    div {
                        class: "form-group",
                        label { class: "form-label", {i18n.t("auth.confirm_password")} }
                        input {
                            class: "form-input",
                            r#type: "password",
                            placeholder: i18n.t("auth.confirm_password_placeholder"),
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary btn-full",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() { {i18n.t("auth.registering")} } else { {i18n.t("auth.register")} }
                    }
                }

                div {
                    class: "auth-footer",
                    {i18n.t("auth.has_account")}
                    button {
                        class: "link",
                        r#type: "button",
                        onclick: move |_| { nav.push(Route::Login {}); },
                        " ",
                        {i18n.t("auth.login_now")}
                    }
                }
            }
        }
    }
}
