use dioxus::prelude::*;
use ui::{Button, ButtonVariant};

use crate::services::{api_client::with_auto_refresh, settings_service};
use crate::stores::auth_store::AuthStore;
use crate::stores::user_store::UserStore;

/// 系统设置页面
///
/// - 普通用户：无此页面入口（个人偏好通过导航栏按鈕切换，存 localStorage）
/// - Admin：全局系统参数配置（调用 SettingsApi，需 Admin token）
#[component]
pub fn Settings() -> Element {
    let user_store = use_context::<UserStore>();
    let auth_store = use_context::<AuthStore>();
    let is_admin = user_store
        .info
        .read()
        .as_ref()
        .map(|u| u.is_admin())
        .unwrap_or(false);

    let settings = use_resource(move || async move {
        with_auto_refresh(auth_store, |token| async move {
            settings_service::get_all(&token).await
        })
        .await
    });

    let save_error = use_signal(String::new);
    let save_ok = use_signal(|| false);

    let get_val = move |key: &str| -> String {
        match settings() {
            Some(Ok(ref m)) => m.get(key).map(|v| v.to_string_value()).unwrap_or_default(),
            _ => String::new(),
        }
    };

    // 使用与后端一致的设置 key 名称
    let platform_name = get_val("site_name");
    let register_mode = get_val("registration_mode");
    let currency = get_val("default_currency");
    let min_recharge = get_val("min_recharge_amount");
    let jwt_expire = get_val("jwt_expire_hours");
    let email_verify = get_val("email_verification_required");

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "系统设置" }
            p { class: "page-description",
                if is_admin { "配置平台全局系统参数" }
                else { "查看系统运行配置（仅供参考）" }
            }
        }

        if !is_admin {
            div { class: "alert alert-info",
                span { class: "alert-icon", "ℹ" }
                div { class: "alert-content",
                    p { class: "alert-body",
                        "系统设置仅 Admin 可修改。个人语言/主题偏好请通过顶部导航栏右侧按鈕切换。"
                    }
                }
            }
        }

        match settings() {
            None => rsx! { p { class: "text-secondary", "加载中..." } },
            Some(Err(_)) => rsx! { p { class: "text-secondary", "设置加载失败" } },
            Some(Ok(_)) => rsx! {
                if save_ok() {
                    div { class: "alert alert-success",
                        span { "✔ 设置已保存" }
                    }
                }
                if !save_error().is_empty() {
                    div { class: "alert alert-error",
                        span { "{save_error}" }
                    }
                }

                // 基础系统配置
                div { class: "card",
                    div { class: "card-header",
                        h3 { class: "card-title", "基础配置" }
                    }
                    div { class: "card-body",
                        div { class: "settings-grid",
                            // 平台名称 - 任意字符串
                            SettingItemText { label: "平台名称", setting_key: "site_name", value: platform_name.clone(), editable: is_admin, auth_store, save_ok, save_error }
                            // 注册模式 - 可选项
                            SettingItemSelect {
                                label: "注册模式",
                                setting_key: "registration_mode",
                                value: register_mode.clone(),
                                editable: is_admin,
                                auth_store,
                                save_ok,
                                save_error,
                                options: vec![
                                    ("open".to_string(), "开放注册".to_string()),
                                    ("invite".to_string(), "邀请注册".to_string()),
                                    ("close".to_string(), "关闭注册".to_string()),
                                ]
                            }
                            // 默认货币 - 可选项
                            SettingItemSelect {
                                label: "默认货币",
                                setting_key: "default_currency",
                                value: currency.clone(),
                                editable: is_admin,
                                auth_store,
                                save_ok,
                                save_error,
                                options: vec![
                                    ("CNY".to_string(), "人民币 (CNY)".to_string()),
                                    ("USD".to_string(), "美元 (USD)".to_string()),
                                ]
                            }
                            // 最低充值金额 - 数字
                            SettingItemNumber { label: "最低充值金额", setting_key: "min_recharge_amount", value: min_recharge.clone(), editable: is_admin, auth_store, save_ok, save_error }
                        }
                    }
                }

                // 安全配置
                div { class: "card",
                    div { class: "card-header",
                        h3 { class: "card-title", "安全配置" }
                    }
                    div { class: "card-body",
                        div { class: "settings-grid",
                            // JWT Token 有效期 - 数字
                            SettingItemNumber { label: "JWT Token 有效期（小时）", setting_key: "jwt_expire_hours", value: jwt_expire.clone(), editable: is_admin, auth_store, save_ok, save_error }
                            // 邮箱验证 - 布尔值
                            SettingItemToggle { label: "邮箱验证", setting_key: "email_verification_required", value: email_verify.clone(), editable: is_admin, auth_store, save_ok, save_error }
                        }
                    }
                }
            },
        }
    }
}

// ── 内部组件

/// 文本输入设置项
#[component]
fn SettingItemText(
    label: String,
    setting_key: String,
    value: String,
    editable: bool,
    auth_store: AuthStore,
    mut save_ok: Signal<bool>,
    mut save_error: Signal<String>,
) -> Element {
    let mut edit_val = use_signal(|| value.clone());
    let mut saving = use_signal(|| false);

    let value_for_effect = value.clone();
    use_effect(move || {
        *edit_val.write() = value_for_effect.clone();
    });

    let key = setting_key.clone();
    let on_save = move |_| {
        let val = edit_val();
        let k = key.clone();
        let token = auth_store.token().unwrap_or_default();
        *saving.write() = true;
        *save_ok.write() = false;
        *save_error.write() = String::new();
        spawn(async move {
            let json_val = serde_json::Value::String(val);
            match settings_service::update_by_key(&k, &json_val, &token).await {
                Ok(_) => {
                    *save_ok.write() = true;
                    *saving.write() = false;
                }
                Err(e) => {
                    *save_error.write() = format!("保存 {} 失败：{}", k, e);
                    *saving.write() = false;
                }
            }
        });
    };

    rsx! {
        div { class: "setting-item",
            span { class: "setting-label", "{label}" }
            if editable {
                div { class: "setting-input-row",
                    input {
                        class: "input-field",
                        value: "{edit_val}",
                        oninput: move |e| *edit_val.write() = e.value(),
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ui::ButtonSize::Small,
                        loading: saving(),
                        onclick: on_save,
                        if saving() { "保存中..." } else { "保存" }
                    }
                }
            } else {
                span { class: "setting-value",
                    if value.is_empty() { "—" } else { "{value}" }
                }
            }
        }
    }
}

/// 数字输入设置项
#[component]
fn SettingItemNumber(
    label: String,
    setting_key: String,
    value: String,
    editable: bool,
    auth_store: AuthStore,
    mut save_ok: Signal<bool>,
    mut save_error: Signal<String>,
) -> Element {
    let mut edit_val = use_signal(|| value.clone());
    let mut saving = use_signal(|| false);
    let mut error_msg = use_signal(String::new);

    let value_for_effect = value.clone();
    use_effect(move || {
        *edit_val.write() = value_for_effect.clone();
    });

    let key = setting_key.clone();
    let on_save = move |_| {
        // 验证输入值
        let val_str = edit_val();
        if let Ok(num) = val_str.parse::<f64>() {
            if num < 0.0 {
                *error_msg.write() = "值不能为负数".to_string();
                return;
            }
            *error_msg.write() = String::new();
        } else {
            *error_msg.write() = "请输入有效的数字".to_string();
            return;
        }

        let k = key.clone();
        let token = auth_store.token().unwrap_or_default();
        *saving.write() = true;
        *save_ok.write() = false;
        *save_error.write() = String::new();
        spawn(async move {
            let json_val = serde_json::Value::String(val_str);
            match settings_service::update_by_key(&k, &json_val, &token).await {
                Ok(_) => {
                    *save_ok.write() = true;
                    *saving.write() = false;
                }
                Err(e) => {
                    *save_error.write() = format!("保存 {} 失败：{}", k, e);
                    *saving.write() = false;
                }
            }
        });
    };

    rsx! {
        div { class: "setting-item",
            span { class: "setting-label", "{label}" }
            if editable {
                div { class: "setting-input-col",
                    div { class: "setting-input-row",
                        input {
                            class: "input-field",
                            r#type: "number",
                            min: "0",
                            value: "{edit_val}",
                            oninput: move |e| {
                                *edit_val.write() = e.value();
                                *error_msg.write() = String::new();
                            },
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            size: ui::ButtonSize::Small,
                            loading: saving(),
                            onclick: on_save,
                            if saving() { "保存中..." } else { "保存" }
                        }
                    }
                    if !error_msg().is_empty() {
                        span { class: "error-text text-sm", "{error_msg}" }
                    }
                }
            } else {
                span { class: "setting-value",
                    if value.is_empty() { "—" } else { "{value}" }
                }
            }
        }
    }
}

/// 下拉选择设置项
#[component]
fn SettingItemSelect(
    label: String,
    setting_key: String,
    value: String,
    editable: bool,
    auth_store: AuthStore,
    mut save_ok: Signal<bool>,
    mut save_error: Signal<String>,
    options: Vec<(String, String)>,
) -> Element {
    let mut edit_val = use_signal(|| value.clone());
    let mut saving = use_signal(|| false);

    let value_for_effect = value.clone();
    use_effect(move || {
        *edit_val.write() = value_for_effect.clone();
    });

    let key = setting_key.clone();
    let on_save = move |_| {
        let val = edit_val();
        let k = key.clone();
        let token = auth_store.token().unwrap_or_default();
        *saving.write() = true;
        *save_ok.write() = false;
        *save_error.write() = String::new();
        spawn(async move {
            let json_val = serde_json::Value::String(val);
            match settings_service::update_by_key(&k, &json_val, &token).await {
                Ok(_) => {
                    *save_ok.write() = true;
                    *saving.write() = false;
                }
                Err(e) => {
                    *save_error.write() = format!("保存 {} 失败：{}", k, e);
                    *saving.write() = false;
                }
            }
        });
    };

    rsx! {
        div { class: "setting-item",
            span { class: "setting-label", "{label}" }
            if editable {
                div { class: "setting-input-row",
                    select {
                        class: "input-field",
                        onchange: move |e| *edit_val.write() = e.value(),
                        for (opt_val, opt_label) in options.iter() {
                            option {
                                value: "{opt_val}",
                                selected: *opt_val == edit_val(),
                                "{opt_label}"
                            }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ui::ButtonSize::Small,
                        loading: saving(),
                        onclick: on_save,
                        if saving() { "保存中..." } else { "保存" }
                    }
                }
            } else {
                span { class: "setting-value",
                    if value.is_empty() { "—" } else { "{value}" }
                }
            }
        }
    }
}

/// 开关切换设置项（布尔值）
#[component]
fn SettingItemToggle(
    label: String,
    setting_key: String,
    value: String,
    editable: bool,
    auth_store: AuthStore,
    mut save_ok: Signal<bool>,
    mut save_error: Signal<String>,
) -> Element {
    let mut edit_val = use_signal(|| value.clone());
    let mut saving = use_signal(|| false);

    let value_for_effect = value.clone();
    use_effect(move || {
        *edit_val.write() = value_for_effect.clone();
    });

    let key = setting_key.clone();
    let mut on_save = move |new_val: String| {
        let k = key.clone();
        let token = auth_store.token().unwrap_or_default();
        *saving.write() = true;
        *save_ok.write() = false;
        *save_error.write() = String::new();
        spawn(async move {
            let json_val = serde_json::Value::String(new_val);
            match settings_service::update_by_key(&k, &json_val, &token).await {
                Ok(_) => {
                    *save_ok.write() = true;
                    *saving.write() = false;
                }
                Err(e) => {
                    *save_error.write() = format!("保存 {} 失败：{}", k, e);
                    *saving.write() = false;
                }
            }
        });
    };

    let is_enabled = edit_val() == "true" || edit_val() == "1";

    rsx! {
        div { class: "setting-item",
            span { class: "setting-label", "{label}" }
            if editable {
                div { class: "setting-input-row",
                    label { class: "toggle-switch",
                        input {
                            r#type: "checkbox",
                            checked: is_enabled,
                            onchange: move |e| {
                                let new_val = if e.checked() { "true" } else { "false" };
                                *edit_val.write() = new_val.to_string();
                                on_save(new_val.to_string());
                            },
                        }
                        span { class: "toggle-slider" }
                    }
                }
            } else {
                span { class: "setting-value",
                    if is_enabled { "已启用" } else { "已禁用" }
                }
            }
        }
    }
}
