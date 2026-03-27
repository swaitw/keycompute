//! 支付宝配置模块

use serde::Deserialize;

/// 支付宝环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AlipayEnv {
    /// 沙箱环境
    Sandbox,
    /// 生产环境
    #[default]
    Production,
}

impl AlipayEnv {
    /// 获取支付宝网关地址
    pub fn gateway_url(&self) -> &'static str {
        match self {
            AlipayEnv::Sandbox => "https://openapi.alipaydev.com/gateway.do",
            AlipayEnv::Production => "https://openapi.alipay.com/gateway.do",
        }
    }

    /// 是否为沙箱环境
    pub fn is_sandbox(&self) -> bool {
        matches!(self, AlipayEnv::Sandbox)
    }
}

/// 支付宝配置
#[derive(Debug, Clone, Deserialize)]
pub struct AlipayConfig {
    /// 应用ID (AppID)
    pub app_id: String,
    /// 应用私钥 (PEM格式)
    pub private_key: String,
    /// 支付宝公钥 (PEM格式，用于验签)
    pub alipay_public_key: String,
    /// 环境
    #[serde(default)]
    pub env: AlipayEnv,
    /// 异步通知地址
    pub notify_url: String,
    /// 同步返回地址
    pub return_url: Option<String>,
    /// 签名类型 (默认RSA2)
    #[serde(default = "default_sign_type")]
    pub sign_type: String,
    /// 字符集 (默认UTF-8)
    #[serde(default = "default_charset")]
    pub charset: String,
    /// 版本 (默认1.0)
    #[serde(default = "default_version")]
    pub version: String,
    /// 支付超时时间（分钟），默认30分钟
    #[serde(default = "default_timeout")]
    pub timeout_minutes: i32,
}

fn default_sign_type() -> String {
    "RSA2".to_string()
}

fn default_charset() -> String {
    "utf-8".to_string()
}

fn default_version() -> String {
    "1.0".to_string()
}

fn default_timeout() -> i32 {
    30
}

impl Default for AlipayConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            private_key: String::new(),
            alipay_public_key: String::new(),
            env: AlipayEnv::default(),
            notify_url: String::new(),
            return_url: None,
            sign_type: default_sign_type(),
            charset: default_charset(),
            version: default_version(),
            timeout_minutes: default_timeout(),
        }
    }
}

impl AlipayConfig {
    /// 从环境变量创建配置
    pub fn from_env() -> Result<Self, ConfigError> {
        let app_id = std::env::var("ALIPAY_APP_ID")
            .map_err(|_| ConfigError::MissingEnvVar("ALIPAY_APP_ID"))?;

        let private_key = std::env::var("ALIPAY_PRIVATE_KEY")
            .map_err(|_| ConfigError::MissingEnvVar("ALIPAY_PRIVATE_KEY"))?;

        let alipay_public_key = std::env::var("ALIPAY_PUBLIC_KEY")
            .map_err(|_| ConfigError::MissingEnvVar("ALIPAY_PUBLIC_KEY"))?;

        let notify_url = std::env::var("ALIPAY_NOTIFY_URL")
            .map_err(|_| ConfigError::MissingEnvVar("ALIPAY_NOTIFY_URL"))?;

        let env_str = std::env::var("ALIPAY_ENV").unwrap_or_else(|_| "production".to_string());
        let env = match env_str.to_lowercase().as_str() {
            "sandbox" | "dev" | "test" => AlipayEnv::Sandbox,
            _ => AlipayEnv::Production,
        };

        let return_url = std::env::var("ALIPAY_RETURN_URL").ok();
        let timeout_minutes = std::env::var("ALIPAY_TIMEOUT_MINUTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Ok(Self {
            app_id,
            private_key: format_private_key(&private_key),
            alipay_public_key: format_public_key(&alipay_public_key),
            env,
            notify_url,
            return_url,
            sign_type: default_sign_type(),
            charset: default_charset(),
            version: default_version(),
            timeout_minutes,
        })
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.app_id.is_empty() {
            return Err(ConfigError::InvalidConfig("app_id 不能为空"));
        }
        if self.private_key.is_empty() {
            return Err(ConfigError::InvalidConfig("private_key 不能为空"));
        }
        if self.alipay_public_key.is_empty() {
            return Err(ConfigError::InvalidConfig("alipay_public_key 不能为空"));
        }
        if self.notify_url.is_empty() {
            return Err(ConfigError::InvalidConfig("notify_url 不能为空"));
        }
        if self.timeout_minutes <= 0 {
            return Err(ConfigError::InvalidConfig("timeout_minutes 必须大于 0"));
        }
        Ok(())
    }

    /// 获取网关URL
    pub fn gateway_url(&self) -> &'static str {
        self.env.gateway_url()
    }
}

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("缺少环境变量: {0}")]
    MissingEnvVar(&'static str),
    #[error("配置无效: {0}")]
    InvalidConfig(&'static str),
}

/// 格式化私钥（添加PEM头尾）
fn format_private_key(key: &str) -> String {
    let key = key
        .replace("-----BEGIN PRIVATE KEY-----", "")
        .replace("-----END PRIVATE KEY-----", "")
        .replace("-----BEGIN RSA PRIVATE KEY-----", "")
        .replace("-----END RSA PRIVATE KEY-----", "")
        .replace(['\\', 'n', '\n', ' '], "");

    format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----",
        key
    )
}

/// 格式化公钥（添加 PEM 头尾）
fn format_public_key(key: &str) -> String {
    let key = key
        .replace("-----BEGIN PUBLIC KEY-----", "")
        .replace("-----END PUBLIC KEY-----", "")
        .replace("-----BEGIN RSA PUBLIC KEY-----", "")
        .replace("-----END RSA PUBLIC KEY-----", "")
        .replace(['\\', 'n', '\n', ' '], "");

    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        key
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alipay_env() {
        let env = AlipayEnv::Sandbox;
        assert!(env.is_sandbox());
        assert_eq!(
            env.gateway_url(),
            "https://openapi.alipaydev.com/gateway.do"
        );

        let env = AlipayEnv::Production;
        assert!(!env.is_sandbox());
        assert_eq!(env.gateway_url(), "https://openapi.alipay.com/gateway.do");
    }

    #[test]
    fn test_default_config() {
        let config = AlipayConfig::default();
        assert_eq!(config.sign_type, "RSA2");
        assert_eq!(config.charset, "utf-8");
        assert_eq!(config.version, "1.0");
        assert_eq!(config.timeout_minutes, 30);
    }

    #[test]
    fn test_format_keys() {
        let raw_key = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC";
        let formatted = format_private_key(raw_key);
        assert!(formatted.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(formatted.contains("-----END PRIVATE KEY-----"));

        let formatted_pub = format_public_key(raw_key);
        assert!(formatted_pub.contains("-----BEGIN PUBLIC KEY-----"));
        assert!(formatted_pub.contains("-----END PUBLIC KEY-----"));
    }
}
