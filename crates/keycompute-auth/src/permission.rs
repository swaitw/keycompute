//! 权限检查
//!
//! 定义系统中的权限和权限检查逻辑。

/// 权限枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    /// 使用 API
    UseApi,
    /// 查看用量
    ViewUsage,
    /// 管理 API Keys
    ManageApiKeys,
    /// 管理用户
    ManageUsers,
    /// 管理租户设置
    ManageTenant,
    /// 查看账单
    ViewBilling,
    /// 管理账单
    ManageBilling,
    /// 管理定价
    ManagePricing,
    /// 管理 Provider 账号
    ManageProviders,
    /// 系统管理员权限
    SystemAdmin,
}

impl Permission {
    /// 获取权限字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::UseApi => "api:use",
            Permission::ViewUsage => "usage:view",
            Permission::ManageApiKeys => "api_keys:manage",
            Permission::ManageUsers => "users:manage",
            Permission::ManageTenant => "tenant:manage",
            Permission::ViewBilling => "billing:view",
            Permission::ManageBilling => "billing:manage",
            Permission::ManagePricing => "pricing:manage",
            Permission::ManageProviders => "providers:manage",
            Permission::SystemAdmin => "system:admin",
        }
    }

    /// 从字符串解析权限
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "api:use" => Some(Permission::UseApi),
            "usage:view" => Some(Permission::ViewUsage),
            "api_keys:manage" => Some(Permission::ManageApiKeys),
            "users:manage" => Some(Permission::ManageUsers),
            "tenant:manage" => Some(Permission::ManageTenant),
            "billing:view" => Some(Permission::ViewBilling),
            "billing:manage" => Some(Permission::ManageBilling),
            "pricing:manage" => Some(Permission::ManagePricing),
            "providers:manage" => Some(Permission::ManageProviders),
            "system:admin" => Some(Permission::SystemAdmin),
            _ => None,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        Self::parse(s)
    }
}

/// 权限检查器
#[derive(Debug, Clone)]
pub struct PermissionChecker;

impl PermissionChecker {
    /// 检查用户是否有权限执行操作
    pub fn check(user_role: &str, user_permissions: &[Permission], required: &Permission) -> bool {
        // 管理员拥有所有权限
        if user_role == "admin" || user_role == "system" {
            return true;
        }

        // 检查具体权限
        user_permissions.contains(required)
    }

    /// 检查是否需要租户隔离
    pub fn requires_tenant_isolation(permission: &Permission) -> bool {
        matches!(
            permission,
            Permission::UseApi
                | Permission::ViewUsage
                | Permission::ManageApiKeys
                | Permission::ViewBilling
        )
    }
}

/// 预定义的角色权限
pub mod roles {
    use super::Permission;

    /// 普通用户权限
    pub fn user() -> Vec<Permission> {
        vec![Permission::UseApi, Permission::ViewUsage]
    }

    /// 租户管理员权限
    pub fn tenant_admin() -> Vec<Permission> {
        vec![
            Permission::UseApi,
            Permission::ViewUsage,
            Permission::ManageApiKeys,
            Permission::ManageUsers,
            Permission::ManageTenant,
            Permission::ViewBilling,
        ]
    }

    /// 系统管理员权限
    pub fn system_admin() -> Vec<Permission> {
        vec![
            Permission::UseApi,
            Permission::ViewUsage,
            Permission::ManageApiKeys,
            Permission::ManageUsers,
            Permission::ManageTenant,
            Permission::ViewBilling,
            Permission::ManagePricing,
            Permission::ManageProviders,
            Permission::SystemAdmin,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_as_str() {
        assert_eq!(Permission::UseApi.as_str(), "api:use");
        assert_eq!(Permission::SystemAdmin.as_str(), "system:admin");
    }

    #[test]
    fn test_permission_from_str() {
        assert_eq!(Permission::parse("api:use"), Some(Permission::UseApi));
        assert_eq!(Permission::parse("invalid"), None);
    }

    #[test]
    fn test_permission_checker_admin() {
        let perms = vec![Permission::UseApi];
        assert!(PermissionChecker::check(
            "admin",
            &perms,
            &Permission::ManageUsers
        ));
    }

    #[test]
    fn test_permission_checker_user() {
        let perms = vec![Permission::UseApi, Permission::ViewUsage];
        assert!(PermissionChecker::check(
            "user",
            &perms,
            &Permission::UseApi
        ));
        assert!(!PermissionChecker::check(
            "user",
            &perms,
            &Permission::ManageUsers
        ));
    }

    #[test]
    fn test_roles() {
        let user_perms = roles::user();
        assert!(user_perms.contains(&Permission::UseApi));
        assert!(!user_perms.contains(&Permission::ManageUsers));

        let admin_perms = roles::tenant_admin();
        assert!(admin_perms.contains(&Permission::ManageApiKeys));
    }
}
