//! 系统设置模型
//!
//! 提供全局系统配置的 CRUD 操作

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 设置值类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SettingValueType {
    String,
    Bool,
    Int,
    Decimal,
    Json,
}

impl From<&str> for SettingValueType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bool" => SettingValueType::Bool,
            "int" => SettingValueType::Int,
            "decimal" => SettingValueType::Decimal,
            "json" => SettingValueType::Json,
            _ => SettingValueType::String,
        }
    }
}

impl std::fmt::Display for SettingValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingValueType::String => write!(f, "string"),
            SettingValueType::Bool => write!(f, "bool"),
            SettingValueType::Int => write!(f, "int"),
            SettingValueType::Decimal => write!(f, "decimal"),
            SettingValueType::Json => write!(f, "json"),
        }
    }
}

/// 系统设置模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SystemSetting {
    pub id: Uuid,
    pub key: String,
    pub value: String,
    pub value_type: String,
    pub description: Option<String>,
    pub is_sensitive: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 系统设置键名常量
pub mod setting_keys {
    // 站点设置
    pub const SITE_NAME: &str = "site_name";
    pub const SITE_DESCRIPTION: &str = "site_description";
    pub const SITE_LOGO_URL: &str = "site_logo_url";
    pub const SITE_FAVICON_URL: &str = "site_favicon_url";

    // 注册设置
    pub const ALLOW_REGISTRATION: &str = "allow_registration";
    pub const EMAIL_VERIFICATION_REQUIRED: &str = "email_verification_required";
    pub const DEFAULT_USER_QUOTA: &str = "default_user_quota";
    pub const DEFAULT_USER_ROLE: &str = "default_user_role";

    // 限流设置
    pub const DEFAULT_RPM_LIMIT: &str = "default_rpm_limit";
    pub const DEFAULT_TPM_LIMIT: &str = "default_tpm_limit";

    // 系统状态
    pub const MAINTENANCE_MODE: &str = "maintenance_mode";
    pub const MAINTENANCE_MESSAGE: &str = "maintenance_message";

    // 分销设置
    pub const DISTRIBUTION_ENABLED: &str = "distribution_enabled";
    pub const DISTRIBUTION_LEVEL1_DEFAULT_RATIO: &str = "distribution_level1_default_ratio";
    pub const DISTRIBUTION_LEVEL2_DEFAULT_RATIO: &str = "distribution_level2_default_ratio";
    pub const DISTRIBUTION_MIN_WITHDRAW: &str = "distribution_min_withdraw";

    // 支付设置
    pub const ALIPAY_ENABLED: &str = "alipay_enabled";
    pub const WECHATPAY_ENABLED: &str = "wechatpay_enabled";
    pub const MIN_RECHARGE_AMOUNT: &str = "min_recharge_amount";
    pub const MAX_RECHARGE_AMOUNT: &str = "max_recharge_amount";

    // 安全设置
    pub const LOGIN_FAILED_LIMIT: &str = "login_failed_limit";
    pub const LOGIN_LOCKOUT_MINUTES: &str = "login_lockout_minutes";
    // 密码策略使用硬编码，参见 keycompute-auth/src/password/validator.rs

    // 公告设置
    pub const SYSTEM_NOTICE: &str = "system_notice";
    pub const SYSTEM_NOTICE_ENABLED: &str = "system_notice_enabled";

    // 其他设置
    pub const FOOTER_CONTENT: &str = "footer_content";
    pub const ABOUT_CONTENT: &str = "about_content";
    pub const TERMS_OF_SERVICE_URL: &str = "terms_of_service_url";
    pub const PRIVACY_POLICY_URL: &str = "privacy_policy_url";
}

/// 更新系统设置请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSystemSettingRequest {
    pub value: String,
}

/// 批量更新系统设置请求
#[derive(Debug, Clone, Deserialize)]
pub struct BatchUpdateSettingsRequest {
    pub settings: std::collections::HashMap<String, String>,
}

/// 系统设置响应（包含类型信息）
#[derive(Debug, Clone, Serialize)]
pub struct SystemSettingResponse {
    pub key: String,
    pub value: String,
    pub value_type: String,
    pub description: Option<String>,
}

impl From<SystemSetting> for SystemSettingResponse {
    fn from(setting: SystemSetting) -> Self {
        Self {
            key: setting.key,
            value: setting.value,
            value_type: setting.value_type,
            description: setting.description,
        }
    }
}

/// 公开系统设置（只包含前端需要的设置）
#[derive(Debug, Clone, Serialize)]
pub struct PublicSettings {
    pub site_name: String,
    pub site_description: Option<String>,
    pub site_logo_url: Option<String>,
    pub site_favicon_url: Option<String>,
    pub allow_registration: bool,
    pub email_verification_required: bool,
    pub maintenance_mode: bool,
    pub maintenance_message: Option<String>,
    pub alipay_enabled: bool,
    pub wechatpay_enabled: bool,
    pub system_notice: Option<String>,
    pub system_notice_enabled: bool,
    pub footer_content: Option<String>,
    pub about_content: Option<String>,
    pub terms_of_service_url: Option<String>,
    pub privacy_policy_url: Option<String>,
}

impl Default for PublicSettings {
    fn default() -> Self {
        Self {
            site_name: "KeyCompute".to_string(),
            site_description: Some("AI Gateway Platform".to_string()),
            site_logo_url: None,
            site_favicon_url: None,
            allow_registration: true,
            email_verification_required: true,
            maintenance_mode: false,
            maintenance_message: None,
            alipay_enabled: false,
            wechatpay_enabled: false,
            system_notice: None,
            system_notice_enabled: false,
            footer_content: None,
            about_content: None,
            terms_of_service_url: None,
            privacy_policy_url: None,
        }
    }
}

impl SystemSetting {
    /// 解析布尔值
    pub fn parse_bool(&self) -> bool {
        self.value.to_lowercase() == "true" || self.value == "1"
    }

    /// 解析整数
    pub fn parse_int(&self) -> Result<i32, std::num::ParseIntError> {
        self.value.parse()
    }

    /// 解析浮点数
    pub fn parse_decimal(&self) -> Result<f64, std::num::ParseFloatError> {
        self.value.parse()
    }

    /// 解析 JSON
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.value)
    }

    /// 根据键名查找设置
    pub async fn find_by_key(
        pool: &sqlx::PgPool,
        key: &str,
    ) -> Result<Option<SystemSetting>, sqlx::Error> {
        let setting =
            sqlx::query_as::<_, SystemSetting>("SELECT * FROM system_settings WHERE key = $1")
                .bind(key)
                .fetch_optional(pool)
                .await?;

        Ok(setting)
    }

    /// 获取所有设置
    pub async fn find_all(pool: &sqlx::PgPool) -> Result<Vec<SystemSetting>, sqlx::Error> {
        let settings =
            sqlx::query_as::<_, SystemSetting>("SELECT * FROM system_settings ORDER BY key ASC")
                .fetch_all(pool)
                .await?;

        Ok(settings)
    }

    /// 获取所有非敏感设置
    pub async fn find_non_sensitive(
        pool: &sqlx::PgPool,
    ) -> Result<Vec<SystemSetting>, sqlx::Error> {
        let settings = sqlx::query_as::<_, SystemSetting>(
            "SELECT * FROM system_settings WHERE is_sensitive = false ORDER BY key ASC",
        )
        .fetch_all(pool)
        .await?;

        Ok(settings)
    }

    /// 更新设置值
    pub async fn update_value(
        pool: &sqlx::PgPool,
        key: &str,
        value: &str,
    ) -> Result<SystemSetting, sqlx::Error> {
        let setting = sqlx::query_as::<_, SystemSetting>(
            r#"
            UPDATE system_settings 
            SET value = $1, updated_at = NOW()
            WHERE key = $2
            RETURNING *
            "#,
        )
        .bind(value)
        .bind(key)
        .fetch_one(pool)
        .await?;

        Ok(setting)
    }

    /// 批量更新设置
    pub async fn batch_update(
        pool: &sqlx::PgPool,
        settings: &std::collections::HashMap<String, String>,
    ) -> Result<Vec<SystemSetting>, sqlx::Error> {
        let mut updated = Vec::new();

        for (key, value) in settings {
            if let Ok(setting) = Self::update_value(pool, key, value).await {
                updated.push(setting);
            }
        }

        Ok(updated)
    }

    /// 获取设置的字符串值，不存在则返回默认值
    pub async fn get_string(pool: &sqlx::PgPool, key: &str, default: &str) -> String {
        match Self::find_by_key(pool, key).await {
            Ok(Some(setting)) => setting.value,
            _ => default.to_string(),
        }
    }

    /// 获取设置的布尔值，不存在则返回默认值
    pub async fn get_bool(pool: &sqlx::PgPool, key: &str, default: bool) -> bool {
        match Self::find_by_key(pool, key).await {
            Ok(Some(setting)) => setting.parse_bool(),
            _ => default,
        }
    }

    /// 获取设置的整数值，不存在则返回默认值
    pub async fn get_int(pool: &sqlx::PgPool, key: &str, default: i32) -> i32 {
        match Self::find_by_key(pool, key).await {
            Ok(Some(setting)) => setting.parse_int().unwrap_or(default),
            _ => default,
        }
    }

    /// 获取设置的浮点数值，不存在则返回默认值
    pub async fn get_decimal(pool: &sqlx::PgPool, key: &str, default: f64) -> f64 {
        match Self::find_by_key(pool, key).await {
            Ok(Some(setting)) => setting.parse_decimal().unwrap_or(default),
            _ => default,
        }
    }

    /// 获取公开设置
    pub async fn get_public_settings(pool: &sqlx::PgPool) -> PublicSettings {
        let settings = Self::find_non_sensitive(pool).await.unwrap_or_default();

        let get_value = |key: &str| -> Option<String> {
            settings
                .iter()
                .find(|s| s.key == key)
                .map(|s| s.value.clone())
        };

        let get_bool_value = |key: &str, default: bool| -> bool {
            settings
                .iter()
                .find(|s| s.key == key)
                .map(|s| s.parse_bool())
                .unwrap_or(default)
        };

        PublicSettings {
            site_name: get_value(setting_keys::SITE_NAME)
                .unwrap_or_else(|| "KeyCompute".to_string()),
            site_description: get_value(setting_keys::SITE_DESCRIPTION),
            site_logo_url: get_value(setting_keys::SITE_LOGO_URL),
            site_favicon_url: get_value(setting_keys::SITE_FAVICON_URL),
            allow_registration: get_bool_value(setting_keys::ALLOW_REGISTRATION, true),
            email_verification_required: get_bool_value(
                setting_keys::EMAIL_VERIFICATION_REQUIRED,
                true,
            ),
            maintenance_mode: get_bool_value(setting_keys::MAINTENANCE_MODE, false),
            maintenance_message: get_value(setting_keys::MAINTENANCE_MESSAGE),
            alipay_enabled: get_bool_value(setting_keys::ALIPAY_ENABLED, false),
            wechatpay_enabled: get_bool_value(setting_keys::WECHATPAY_ENABLED, false),
            system_notice: get_value(setting_keys::SYSTEM_NOTICE),
            system_notice_enabled: get_bool_value(setting_keys::SYSTEM_NOTICE_ENABLED, false),
            footer_content: get_value(setting_keys::FOOTER_CONTENT),
            about_content: get_value(setting_keys::ABOUT_CONTENT),
            terms_of_service_url: get_value(setting_keys::TERMS_OF_SERVICE_URL),
            privacy_policy_url: get_value(setting_keys::PRIVACY_POLICY_URL),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_from_str() {
        assert_eq!(SettingValueType::from("string"), SettingValueType::String);
        assert_eq!(SettingValueType::from("bool"), SettingValueType::Bool);
        assert_eq!(SettingValueType::from("int"), SettingValueType::Int);
        assert_eq!(SettingValueType::from("decimal"), SettingValueType::Decimal);
        assert_eq!(SettingValueType::from("json"), SettingValueType::Json);
    }

    #[test]
    fn test_parse_bool() {
        let setting = SystemSetting {
            id: Uuid::nil(),
            key: "test".to_string(),
            value: "true".to_string(),
            value_type: "bool".to_string(),
            description: None,
            is_sensitive: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(setting.parse_bool());

        let setting = SystemSetting {
            value: "false".to_string(),
            ..setting
        };
        assert!(!setting.parse_bool());
    }
}
