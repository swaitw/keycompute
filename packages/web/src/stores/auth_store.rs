use dioxus::prelude::*;

/// 认证状态
#[derive(Clone, PartialEq, Default)]
pub struct AuthState {
    /// 访问令牌
    pub access_token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 是否已登录
    pub is_authenticated: bool,
}

impl AuthState {
    pub fn logged_in(access_token: String, refresh_token: String) -> Self {
        Self {
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            is_authenticated: true,
        }
    }

    #[allow(dead_code)]
    pub fn token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }
}

/// 认证状态 Store（对外暴露的 Signal 封装）
#[derive(Clone, Copy, PartialEq)]
pub struct AuthStore {
    pub state: Signal<AuthState>,
}

impl AuthStore {
    pub fn new() -> Self {
        // 尝试从 localStorage 恢复 token
        let initial = Self::load_from_storage();
        Self {
            state: use_signal(|| initial),
        }
    }

    pub fn login(&mut self, access_token: String, refresh_token: String) {
        Self::save_to_storage(&access_token, &refresh_token);
        *self.state.write() = AuthState::logged_in(access_token, refresh_token);
    }

    pub fn logout(&mut self) {
        Self::clear_storage();
        *self.state.write() = AuthState::default();
    }

    pub fn is_authenticated(&self) -> bool {
        (self.state)().is_authenticated
    }

    pub fn token(&self) -> Option<String> {
        (self.state)().access_token.clone()
    }

    pub fn refresh_token(&self) -> Option<String> {
        (self.state)().refresh_token.clone()
    }

    fn load_from_storage() -> AuthState {
        #[cfg(target_arch = "wasm32")]
        {
            let token = read_local_storage("access_token");
            let refresh = read_local_storage("refresh_token");
            if let (Some(access_token), Some(refresh_token)) = (token, refresh) {
                return AuthState::logged_in(access_token, refresh_token);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some((access, refresh)) = read_native_storage() {
                return AuthState::logged_in(access, refresh);
            }
        }
        AuthState::default()
    }

    fn save_to_storage(access_token: &str, refresh_token: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = write_local_storage("access_token", access_token);
            let _ = write_local_storage("refresh_token", refresh_token);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            write_native_storage(access_token, refresh_token);
        }
    }

    fn clear_storage() {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.remove_item("access_token");
                    let _ = storage.remove_item("refresh_token");
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            clear_native_storage();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn read_local_storage(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

#[cfg(target_arch = "wasm32")]
fn write_local_storage(key: &str, value: &str) -> Option<()> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .set_item(key, value)
        .ok()
}

// ── 非 WASM 环境（桌面端）使用系统临时目录下的 JSON 文件持久化 Token ──

/// 获取格令存储文件路径
#[cfg(not(target_arch = "wasm32"))]
fn native_storage_path() -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push("keycompute_auth.json");
    path
}

/// 从文件读取 (access_token, refresh_token)
#[cfg(not(target_arch = "wasm32"))]
fn read_native_storage() -> Option<(String, String)> {
    let path = native_storage_path();
    let data = std::fs::read_to_string(&path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&data).ok()?;
    let access = parsed["access_token"].as_str()?.to_string();
    let refresh = parsed["refresh_token"].as_str()?.to_string();
    if access.is_empty() || refresh.is_empty() {
        return None;
    }
    Some((access, refresh))
}

/// 将 token 写入文件
#[cfg(not(target_arch = "wasm32"))]
fn write_native_storage(access_token: &str, refresh_token: &str) {
    let path = native_storage_path();
    let content = format!(
        r#"{{"access_token":"{}","refresh_token":"{}"}}
"#,
        access_token, refresh_token
    );
    let _ = std::fs::write(&path, content);
}

/// 删除令牌文件
#[cfg(not(target_arch = "wasm32"))]
fn clear_native_storage() {
    let path = native_storage_path();
    let _ = std::fs::remove_file(&path);
}
