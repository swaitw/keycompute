use dioxus::prelude::*;

use crate::router::Route;
use crate::services::auth_service;

/// 邮箱验证页面
/// 路由：/auth/verify-email/:token
/// 挂载后自动调用验证接口，无需用户操作
#[component]
pub fn VerifyEmail(token: String) -> Element {
    let nav = use_navigator();

    // 页面挂载后自动发起验证请求
    let verify_result = use_resource(move || {
        let token = token.clone();
        async move { auth_service::verify_email(&token).await }
    });

    rsx! {
        div {
            class: "auth-page",
            div {
                class: "auth-card",
                h1 { class: "auth-title", "邮箱验证" }

                match verify_result() {
                    None => rsx! {
                        div { class: "verify-loading",
                            p { "正在验证邮箱，请稍候..." }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        div { class: "alert alert-success",
                            p { "邮箱验证成功！您现在可以登录了。" }
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| { nav.push(Route::Login {}); },
                                "前往登录"
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "alert alert-error",
                            p { "验证失败：{e}" }
                            p { class: "text-secondary", "链接可能已过期，请重新注册或联系支持。" }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| { nav.push(Route::Login {}); },
                                "返回登录"
                            }
                        }
                    },
                }
            }
        }
    }
}
