use dioxus::prelude::*;

use client_api::api::payment::CreatePaymentOrderRequest;

use crate::services::payment_service;
use crate::stores::auth_store::AuthStore;

#[component]
pub fn Recharge() -> Element {
    let auth_store = use_context::<AuthStore>();
    let mut amount = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut success_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        let amount_str = amount();
        if amount_str.is_empty() {
            error_msg.set(Some("请输入充值金额".to_string()));
            return;
        }
        let amount_val: f64 = match amount_str.parse() {
            Ok(v) if v > 0.0 => v,
            _ => {
                error_msg.set(Some("请输入有效金额".to_string()));
                return;
            }
        };
        loading.set(true);
        error_msg.set(None);
        success_msg.set(None);
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            let req = CreatePaymentOrderRequest::new(amount_val, "CNY", "manual");
            match payment_service::create_order(req, &token).await {
                Ok(order) => {
                    success_msg.set(Some(format!(
                        "订单已创建，订单号：{}，请完成支付",
                        order.out_trade_no
                    )));
                    amount.set(String::new());
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("创建订单失败：{e}")));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "充值" }
            }
            div {
                class: "card",
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
                        label { class: "form-label", "充值金额（元）" }
                        div {
                            class: "amount-presets",
                            for preset in ["10", "50", "100", "500"] {
                                button {
                                    class: "btn btn-outline",
                                    r#type: "button",
                                    onclick: move |_| amount.set(preset.to_string()),
                                    "¥{preset}"
                                }
                            }
                        }
                        input {
                            class: "form-input",
                            r#type: "number",
                            placeholder: "或输入自定义金额",
                            value: "{amount}",
                            oninput: move |e| amount.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary btn-full",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() { "处理中..." } else { "确认充值" }
                    }
                }
            }
        }
    }
}
