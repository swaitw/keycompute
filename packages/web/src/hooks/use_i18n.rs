use dioxus::prelude::*;

use crate::i18n::{I18n, Lang};
use ui::layout::app_shell::UiState;

/// 获取国际化实例
/// 优先从 UiState 读取语言，如果没有则使用默认中文
pub fn use_i18n() -> I18n {
    // 尝试从 UiState 获取语言
    let lang = try_use_context::<UiState>()
        .map(|ui_state| {
            let lang_str = (ui_state.lang)();
            match lang_str.as_str() {
                "en" => Lang::En,
                _ => Lang::Zh,
            }
        })
        .unwrap_or(Lang::Zh); // 默认中文

    I18n::new(lang)
}
