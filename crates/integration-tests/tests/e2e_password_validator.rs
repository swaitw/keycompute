//! 密码验证器端到端测试
//!
//! 验证密码和邮箱验证功能的完整测试：
//! - 密码强度验证
//! - 邮箱格式验证
//! - 自定义验证规则
//! - 边界条件测试

use integration_tests::common::VerificationChain;
use keycompute_auth::password::{EmailValidator, PasswordValidator};

/// 测试密码验证器默认配置
#[test]
fn test_password_validator_default() {
    let mut chain = VerificationChain::new();
    let validator = PasswordValidator::new();

    // 1. 有效密码（满足所有条件）
    let valid_passwords = vec![
        "SecurePass123!",
        "MyP@ssw0rd2024",
        "C0mpl3x!Pass",
        "Abcdefg1!",
        "Test@1234",
    ];

    for pwd in valid_passwords {
        let result = validator.validate(pwd);
        chain.add_step(
            "keycompute-auth",
            "validate_valid",
            format!("密码 '{}' 验证通过", &pwd[..8]),
            result.is_ok(),
        );
    }

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试密码长度验证
#[test]
fn test_password_length_validation() {
    let mut chain = VerificationChain::new();
    let validator = PasswordValidator::new();

    // 1. 边界长度测试
    let exact_min = "Ab1!defg"; // 8 characters
    let below_min = "Ab1!def"; // 7 characters
    let above_min = "Ab1!defghij"; // 11 characters

    chain.add_step(
        "keycompute-auth",
        "length_exact_min",
        format!("最小长度(8): {}", exact_min.len()),
        validator.validate(exact_min).is_ok(),
    );

    chain.add_step(
        "keycompute-auth",
        "length_below_min",
        format!("低于最小长度(7): {}", below_min.len()),
        validator.validate(below_min).is_err(),
    );

    chain.add_step(
        "keycompute-auth",
        "length_above_min",
        format!("超过最小长度(11): {}", above_min.len()),
        validator.validate(above_min).is_ok(),
    );

    // 2. 长密码测试
    let long_password = "A1!".to_owned() + &"a".repeat(100);
    chain.add_step(
        "keycompute-auth",
        "length_long_password",
        "长密码 (103): 验证通过".to_string(),
        validator.validate(&long_password).is_ok(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试密码字符类型要求
#[test]
fn test_password_character_types() {
    let mut chain = VerificationChain::new();
    let validator = PasswordValidator::new();

    // 1. 缺少大写字母
    chain.add_step(
        "keycompute-auth",
        "missing_uppercase",
        "缺少大写: 'securepass123!'",
        validator.validate("securepass123!").is_err(),
    );

    // 2. 缺少小写字母
    chain.add_step(
        "keycompute-auth",
        "missing_lowercase",
        "缺少小写: 'SECUREPASS123!'",
        validator.validate("SECUREPASS123!").is_err(),
    );

    // 3. 缺少数字
    chain.add_step(
        "keycompute-auth",
        "missing_digit",
        "缺少数字: 'SecurePass!!'",
        validator.validate("SecurePass!!").is_err(),
    );

    // 4. 缺少特殊字符
    chain.add_step(
        "keycompute-auth",
        "missing_special",
        "缺少特殊字符: 'SecurePass123'",
        validator.validate("SecurePass123").is_err(),
    );

    // 5. 同时缺少多种类型
    chain.add_step(
        "keycompute-auth",
        "missing_multiple",
        "缺少多种: 'password'",
        validator.validate("password").is_err(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试密码验证器自定义配置
#[test]
fn test_password_validator_custom() {
    let mut chain = VerificationChain::new();

    // 1. 仅检查长度的宽松模式
    let lenient = PasswordValidator::lenient();
    chain.add_step(
        "keycompute-auth",
        "lenient_valid",
        "宽松模式: 'simplepassword' 通过",
        lenient.validate("simplepassword").is_ok(),
    );
    chain.add_step(
        "keycompute-auth",
        "lenient_invalid",
        "宽松模式: 'short' 拒绝",
        lenient.validate("short").is_err(),
    );

    // 2. 自定义最小长度
    let custom_length = PasswordValidator::new().with_min_length(12);
    chain.add_step(
        "keycompute-auth",
        "custom_length_valid",
        "最小长度12: 'SecurePass123!' 通过",
        custom_length.validate("SecurePass123!").is_ok(),
    );
    chain.add_step(
        "keycompute-auth",
        "custom_length_invalid",
        "最小长度12: 'Secure1!' 拒绝",
        custom_length.validate("Secure1!").is_err(),
    );

    // 3. 禁用特殊字符要求
    let no_special = PasswordValidator::new().with_special(false);
    chain.add_step(
        "keycompute-auth",
        "no_special_valid",
        "无特殊字符要求: 'SecurePass123' 通过",
        no_special.validate("SecurePass123").is_ok(),
    );

    // 4. 禁用数字要求
    let no_digit = PasswordValidator::new().with_digit(false);
    chain.add_step(
        "keycompute-auth",
        "no_digit_valid",
        "无数字要求: 'SecurePass!!' 通过",
        no_digit.validate("SecurePass!!").is_ok(),
    );

    // 5. 组合自定义
    let combined = PasswordValidator::new()
        .with_min_length(6)
        .with_uppercase(false)
        .with_special(false);
    chain.add_step(
        "keycompute-auth",
        "combined_custom",
        "组合自定义: 'simple123' 通过",
        combined.validate("simple123").is_ok(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试密码快速验证方法
#[test]
fn test_password_is_valid_quick() {
    let mut chain = VerificationChain::new();
    let validator = PasswordValidator::new();

    // 1. 快速验证有效密码
    chain.add_step(
        "keycompute-auth",
        "is_valid_true",
        "快速验证: 'SecurePass123!' 为 true",
        validator.is_valid("SecurePass123!"),
    );

    // 2. 快速验证无效密码
    chain.add_step(
        "keycompute-auth",
        "is_valid_false",
        "快速验证: 'weak' 为 false",
        !validator.is_valid("weak"),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试邮箱验证器
#[test]
fn test_email_validator_valid() {
    let mut chain = VerificationChain::new();
    let validator = EmailValidator::new();

    // 1. 标准邮箱格式
    let valid_emails = vec![
        "user@example.com",
        "test.email@domain.org",
        "user+tag@example.co.uk",
        "user123@test-domain.com",
        "first.last@company.io",
        "user_name@example.com",
        "user%test@example.com",
        "123@numeric.com",
        "a@b.co",
    ];

    for email in valid_emails {
        let result = validator.validate(email);
        chain.add_step(
            "keycompute-auth",
            "email_valid",
            format!("邮箱 '{}' 验证通过", email),
            result.is_ok(),
        );
    }

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试邮箱验证器拒绝无效格式
#[test]
fn test_email_validator_invalid() {
    let mut chain = VerificationChain::new();
    let validator = EmailValidator::new();

    // 1. 无效邮箱格式
    let invalid_emails = vec![
        ("", "空字符串"),
        ("invalid", "无@符号"),
        ("user@", "无域名"),
        ("@example.com", "无用户名"),
        ("user@example", "无顶级域名"),
        ("user @example.com", "包含空格"),
        ("user@@example.com", "双@符号"),
        ("user@exam ple.com", "域名空格"),
        ("user@.com", "域名以点开头"),
    ];

    for (email, reason) in invalid_emails {
        let result = validator.validate(email);
        chain.add_step(
            "keycompute-auth",
            "email_invalid",
            format!("拒绝 '{}': {}", email, reason),
            result.is_err(),
        );
    }

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试邮箱规范化
#[test]
fn test_email_normalization() {
    let mut chain = VerificationChain::new();
    let validator = EmailValidator::new();

    // 1. 大小写转换
    chain.add_step(
        "keycompute-auth",
        "normalize_lowercase",
        "'User@Example.COM' -> 'user@example.com'",
        validator.normalize("User@Example.COM") == "user@example.com",
    );

    // 2. 去除首尾空格
    chain.add_step(
        "keycompute-auth",
        "normalize_trim",
        "'  user@example.com  ' -> 'user@example.com'",
        validator.normalize("  user@example.com  ") == "user@example.com",
    );

    // 3. 组合情况
    chain.add_step(
        "keycompute-auth",
        "normalize_combined",
        "'  USER@EXAMPLE.COM  ' -> 'user@example.com'",
        validator.normalize("  USER@EXAMPLE.COM  ") == "user@example.com",
    );

    // 4. 规范化后验证
    let normalized = validator.normalize("  User@Example.COM  ");
    chain.add_step(
        "keycompute-auth",
        "validate_normalized",
        "规范化后的邮箱验证通过",
        validator.validate(&normalized).is_ok(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试邮箱长度限制
#[test]
fn test_email_length_validation() {
    let mut chain = VerificationChain::new();
    let validator = EmailValidator::new();

    // 1. 正常长度邮箱
    let normal_email = "user@example.com";
    chain.add_step(
        "keycompute-auth",
        "email_normal_length",
        format!("正常长度({}): 通过", normal_email.len()),
        validator.validate(normal_email).is_ok(),
    );

    // 2. 超长邮箱（超过255字符）
    let long_local = "a".repeat(250);
    let long_email = format!("{}@example.com", long_local);
    chain.add_step(
        "keycompute-auth",
        "email_too_long",
        format!("超长邮箱({}): 拒绝", long_email.len()),
        validator.validate(&long_email).is_err(),
    );

    // 3. 边界测试（刚好255字符）
    let boundary_local = "a".repeat(243); // 243 + 12 (@example.com) = 255
    let boundary_email = format!("{}@example.com", boundary_local);
    chain.add_step(
        "keycompute-auth",
        "email_boundary",
        format!("边界长度({}): 通过", boundary_email.len()),
        validator.validate(&boundary_email).is_ok(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试邮箱快速验证
#[test]
fn test_email_is_valid_quick() {
    let mut chain = VerificationChain::new();
    let validator = EmailValidator::new();

    // 1. 快速验证有效邮箱
    chain.add_step(
        "keycompute-auth",
        "email_is_valid_true",
        "快速验证: 'user@example.com' 为 true",
        validator.is_valid("user@example.com"),
    );

    // 2. 快速验证无效邮箱
    chain.add_step(
        "keycompute-auth",
        "email_is_valid_false",
        "快速验证: 'invalid' 为 false",
        !validator.is_valid("invalid"),
    );

    // 3. 空邮箱
    chain.add_step(
        "keycompute-auth",
        "email_is_valid_empty",
        "快速验证: '' 为 false",
        !validator.is_valid(""),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试特殊字符密码
#[test]
fn test_password_special_characters() {
    let mut chain = VerificationChain::new();
    let validator = PasswordValidator::new();

    // 1. 各种特殊字符组合
    let special_passwords = vec![
        "Pass123!@#",
        "Pass123$%^",
        "Pass123&*()",
        "Pass123_+-=",
        "Pass123[]{}|",
        "Pass123;':\",./<>?",
        "Pass123`~",
    ];

    for pwd in special_passwords {
        let result = validator.validate(pwd);
        chain.add_step(
            "keycompute-auth",
            "special_char",
            format!("特殊字符 '{}' 通过", &pwd[7..9]),
            result.is_ok(),
        );
    }

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试验证器默认实现
#[test]
fn test_validator_default_impl() {
    let mut chain = VerificationChain::new();

    // 1. PasswordValidator 默认
    let default_pwd: PasswordValidator = Default::default();
    chain.add_step(
        "keycompute-auth",
        "password_default",
        "PasswordValidator Default 创建成功",
        default_pwd.is_valid("SecurePass123!"),
    );

    // 2. EmailValidator 默认
    let default_email: EmailValidator = Default::default();
    chain.add_step(
        "keycompute-auth",
        "email_default",
        "EmailValidator Default 创建成功",
        default_email.is_valid("user@example.com"),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试验证错误信息
#[test]
fn test_validation_error_messages() {
    let mut chain = VerificationChain::new();
    let pwd_validator = PasswordValidator::new();
    let email_validator = EmailValidator::new();

    // 1. 密码错误信息
    let pwd_result = pwd_validator.validate("weak");
    if let Err(e) = pwd_result {
        let error_msg = format!("{}", e);
        chain.add_step(
            "keycompute-auth",
            "password_error_message",
            format!("密码错误包含长度提示: {}", error_msg.contains("长度")),
            error_msg.contains("长度"),
        );
    }

    // 2. 邮箱错误信息
    let email_result = email_validator.validate("invalid");
    if let Err(e) = email_result {
        let error_msg = format!("{}", e);
        chain.add_step(
            "keycompute-auth",
            "email_error_message",
            format!("邮箱错误包含格式提示: {}", error_msg.contains("格式")),
            error_msg.contains("格式"),
        );
    }

    // 3. 空邮箱错误信息
    let empty_result = email_validator.validate("");
    if let Err(e) = empty_result {
        let error_msg = format!("{}", e);
        chain.add_step(
            "keycompute-auth",
            "empty_email_error",
            format!("空邮箱错误包含不能为空: {}", error_msg.contains("不能为空")),
            error_msg.contains("不能为空"),
        );
    }

    chain.print_report();
    assert!(chain.all_passed());
}
