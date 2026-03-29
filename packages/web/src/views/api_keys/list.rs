use dioxus::prelude::*;

use crate::services::api_key_service;
use crate::stores::auth_store::AuthStore;

#[component]
pub fn ApiKeyList() -> Element {
    let auth_store = use_context::<AuthStore>();
    let mut show_create = use_signal(|| false);
    let mut new_key_name = use_signal(String::new);
    let mut creating = use_signal(|| false);
    let mut create_error = use_signal(|| Option::<String>::None);
    let mut new_key_value = use_signal(|| Option::<String>::None); // 新建成功后展示一次完整 key

    // 拉取 key 列表
    let mut keys = use_resource(move || async move {
        let token = auth_store.token().unwrap_or_default();
        api_key_service::list(&token).await
    });

    let on_create = move |evt: Event<FormData>| {
        evt.prevent_default();
        let name = new_key_name();
        if name.is_empty() {
            return;
        }
        creating.set(true);
        create_error.set(None);
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            match api_key_service::create(&name, &token).await {
                Ok(resp) => {
                    new_key_value.set(Some(resp.api_key));
                    show_create.set(false);
                    new_key_name.set(String::new());
                    creating.set(false);
                    // 重新拉取列表
                    keys.restart();
                }
                Err(e) => {
                    create_error.set(Some(format!("创建失败：{e}")));
                    creating.set(false);
                }
            }
        });
    };

    let on_delete = move |id: String| {
        spawn(async move {
            let token = auth_store.token().unwrap_or_default();
            if api_key_service::delete(&id, &token).await.is_ok() {
                keys.restart();
            }
        });
    };

    rsx! {
        div {
            class: "page-container",
            div {
                class: "page-header",
                h1 { class: "page-title", "API Key 管理" }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        show_create.set(true);
                        new_key_value.set(None);
                    },
                    "+ 创建 API Key"
                }
            }

            // 新建成功后展示完整密钥（仅一次）
            if let Some(key) = new_key_value() {
                div {
                    class: "alert alert-success",
                    p { "API Key 已创建，请妥善保存（仅显示一次）：" }
                    code { class: "key-display", "{key}" }
                    button {
                        class: "btn btn-sm btn-ghost",
                        onclick: move |_| new_key_value.set(None),
                        "我已记录，关闭"
                    }
                }
            }

            // 创建弹窗
            if show_create() {
                div {
                    class: "modal-overlay",
                    div {
                        class: "modal",
                        h2 { class: "modal-title", "创建 API Key" }
                        if let Some(err) = create_error() {
                            div { class: "alert alert-error", "{err}" }
                        }
                        form {
                            onsubmit: on_create,
                            div {
                                class: "form-group",
                                label { class: "form-label", "名称" }
                                input {
                                    class: "form-input",
                                    r#type: "text",
                                    placeholder: "为此 Key 取个名字",
                                    value: "{new_key_name}",
                                    oninput: move |e| new_key_name.set(e.value()),
                                }
                            }
                            div {
                                class: "modal-actions",
                                button {
                                    class: "btn btn-ghost",
                                    r#type: "button",
                                    onclick: move |_| show_create.set(false),
                                    "取消"
                                }
                                button {
                                    class: "btn btn-primary",
                                    r#type: "submit",
                                    disabled: creating(),
                                    if creating() { "创建中..." } else { "创建" }
                                }
                            }
                        }
                    }
                }
            }

            div {
                class: "table-container",
                match keys() {
                    None => rsx! {
                        div { class: "loading-state", "加载中..." }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "alert alert-error", "加载失败：{e}" }
                    },
                    Some(Ok(list)) => {
                        if list.is_empty() {
                            rsx! {
                                div {
                                    class: "empty-state",
                                    p { "暂无 API Key，点击上方按钮创建" }
                                }
                            }
                        } else {
                            rsx! {
                                table {
                                    class: "table",
                                    thead {
                                        tr {
                                            th { "名称" }
                                            th { "前缀" }
                                            th { "状态" }
                                            th { "创建时间" }
                                            th { "操作" }
                                        }
                                    }
                                    tbody {
                                        for key in list.iter() {
                                            tr {
                                                key: "{key.id}",
                                                td { "{key.name}" }
                                                td { code { "{key.key_preview}..." } }
                                                td {
                                                    span {
                                                        class: if key.revoked { "badge badge-error" } else { "badge badge-success" },
                                                        if key.revoked { "已撤销" } else { "活跃" }
                                                    }
                                                }
                                                td { "{key.created_at}" }
                                                td {
                                                    button {
                                                        class: "btn btn-sm btn-danger",
                                                        onclick: {
                                                            let id = key.id.to_string();
                                                            move |_| on_delete(id.clone())
                                                        },
                                                        "删除"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
