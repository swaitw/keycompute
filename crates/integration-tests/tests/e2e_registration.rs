//! 用户注册服务端到端测试
//!
//! 验证注册流程的完整功能：
//! - 用户注册
//! - 邮箱验证
//! - 重新发送验证邮件
//! - 邮件发送集成
//! - 错误处理

use integration_tests::common::VerificationChain;
use integration_tests::mocks::database::{
    MockEmailVerification, MockTenant, MockUser, MockUserCredential, MockUserTenantDatabase,
};
use integration_tests::mocks::email::MockEmailService;
use keycompute_auth::password::{EmailValidator, PasswordHasher, PasswordValidator};
use uuid::Uuid;

/// 测试邮箱验证器
#[test]
fn test_email_validator() {
    let mut chain = VerificationChain::new();
    let validator = EmailValidator::new();

    // 1. 验证有效邮箱
    let valid_emails = vec![
        "user@example.com",
        "test.user@domain.org",
        "user+tag@example.co.uk",
        "123@numeric.com",
    ];

    for email in valid_emails {
        let result = validator.validate(email);
        chain.add_step(
            "keycompute-auth",
            "EmailValidator::validate",
            format!("邮箱 {} 验证通过", email),
            result.is_ok(),
        );
    }

    // 2. 验证无效邮箱
    let invalid_emails = vec![
        "invalid",
        "@example.com",
        "user@",
        "user@.com",
        "",
        "user@@example.com",
    ];

    for email in invalid_emails {
        let result = validator.validate(email);
        chain.add_step(
            "keycompute-auth",
            "EmailValidator::validate_invalid",
            format!("邮箱 {} 验证失败", email),
            result.is_err(),
        );
    }

    // 3. 测试邮箱规范化
    let normalized = validator.normalize("  User@Example.COM  ");
    chain.add_step(
        "keycompute-auth",
        "EmailValidator::normalize",
        format!("规范化结果: {}", normalized),
        normalized == "user@example.com",
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试密码验证器
#[test]
fn test_password_validator() {
    let mut chain = VerificationChain::new();
    let validator = PasswordValidator::new();

    // 1. 验证有效密码
    let valid_passwords = vec![
        "StrongPass123!",
        "MyP@ssw0rd",
        "C0mplex!Pass",
        "LongEnough123!",
    ];

    for pwd in valid_passwords {
        let result = validator.validate(pwd);
        chain.add_step(
            "keycompute-auth",
            "PasswordValidator::validate_valid",
            format!("密码长度 {} 验证通过", pwd.len()),
            result.is_ok(),
        );
    }

    // 2. 验证无效密码
    let invalid_passwords = vec![
        ("short", "太短"),
        ("12345678", "没有字母"),
        ("password", "没有数字和大写"),
        ("PASSWORD123", "没有小写"),
    ];

    for (pwd, reason) in invalid_passwords {
        let result = validator.validate(pwd);
        chain.add_step(
            "keycompute-auth",
            "PasswordValidator::validate_invalid",
            format!("{}: {}", reason, pwd),
            result.is_err(),
        );
    }

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试密码哈希器
#[test]
fn test_password_hasher() {
    let mut chain = VerificationChain::new();
    let hasher = PasswordHasher::new();

    let password = "MySecureP@ssw0rd123";

    // 1. 哈希密码
    let hash = hasher.hash(password).unwrap();
    chain.add_step(
        "keycompute-auth",
        "PasswordHasher::hash",
        format!("哈希长度: {}", hash.len()),
        hash.len() > 50, // Argon2 哈希通常很长
    );

    // 2. 验证正确密码
    let verify_result = hasher.verify(password, &hash).unwrap();
    chain.add_step(
        "keycompute-auth",
        "PasswordHasher::verify_correct",
        "正确密码验证通过",
        verify_result,
    );

    // 3. 验证错误密码
    let verify_wrong = hasher.verify("WrongPassword", &hash).unwrap();
    chain.add_step(
        "keycompute-auth",
        "PasswordHasher::verify_wrong",
        "错误密码验证失败",
        !verify_wrong,
    );

    // 4. 不同密码产生不同哈希
    let hash2 = hasher.hash(password).unwrap();
    chain.add_step(
        "keycompute-auth",
        "PasswordHasher::unique_hash",
        "相同密码产生不同哈希（加盐）",
        hash != hash2,
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试 Mock 数据库用户注册流程
#[test]
fn test_mock_registration_flow() {
    let mut chain = VerificationChain::new();
    let db = MockUserTenantDatabase::new();

    // 1. 创建租户
    let tenant = db.create_test_tenant();
    chain.add_step(
        "integration-tests",
        "create_tenant",
        format!("租户创建: {}", tenant.id),
        true,
    );

    // 2. 创建用户
    let user = db.create_test_user(tenant.id, "user");
    chain.add_step(
        "integration-tests",
        "create_user",
        format!("用户创建: {} ({})", user.id, user.email),
        user.tenant_id == tenant.id,
    );

    // 3. 创建用户凭证
    let credential = MockUserCredential::new(user.id, "hashed_password_hash");
    chain.add_step(
        "integration-tests",
        "create_credential",
        format!("凭证创建: {}", credential.id),
        credential.user_id == user.id,
    );

    // 4. 创建邮箱验证记录
    let verification = MockEmailVerification::new(user.id, &user.email, "verify_token_123");
    chain.add_step(
        "integration-tests",
        "create_verification",
        format!("验证记录创建: {}", verification.id),
        verification.user_id == user.id && !verification.used,
    );

    // 5. 验证令牌有效性
    chain.add_step(
        "integration-tests",
        "verification::is_valid",
        "验证令牌有效",
        verification.is_valid(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试 Mock 邮件服务与注册流程集成
#[tokio::test]
async fn test_registration_with_mock_email() {
    let mut chain = VerificationChain::new();
    let email_service = MockEmailService::new();

    // 1. 模拟注册流程
    let email = "newuser@example.com";
    let verification_token = "verify_token_abc123";

    // 2. 发送验证邮件
    let result = email_service
        .send_verification_email(email, verification_token)
        .await;
    chain.add_step(
        "integration-tests",
        "send_verification_email",
        "验证邮件发送成功",
        result.is_ok(),
    );

    // 3. 验证邮件已记录
    chain.add_step(
        "integration-tests",
        "email_recorded",
        "验证邮件已记录",
        email_service.has_verification_email(email),
    );

    // 4. 验证邮件内容
    let emails = email_service.get_emails_to(email);
    chain.add_step(
        "integration-tests",
        "email_content",
        format!("邮件主题: {}", emails[0].subject),
        emails[0].subject.contains("验证"),
    );

    // 5. 验证令牌关联
    let email_record = email_service.get_email_by_token(verification_token);
    chain.add_step(
        "integration-tests",
        "token_association",
        "邮件与令牌关联正确",
        email_record.is_some() && email_record.unwrap().to == email,
    );

    // 6. 模拟邮箱验证成功后发送欢迎邮件
    let welcome_result = email_service
        .send_welcome_email(email, Some("新用户"))
        .await;
    chain.add_step(
        "integration-tests",
        "send_welcome_email",
        "欢迎邮件发送成功",
        welcome_result.is_ok(),
    );

    chain.add_step(
        "integration-tests",
        "welcome_email_recorded",
        "欢迎邮件已记录",
        email_service.has_welcome_email(email),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试注册请求验证
#[test]
fn test_register_request_validation() {
    let mut chain = VerificationChain::new();

    // 1. 测试邮箱验证器
    let email_validator = EmailValidator::new();

    // 有效邮箱
    let valid_email = "user@example.com";
    chain.add_step(
        "keycompute-auth",
        "validate_email_valid",
        format!("验证邮箱: {}", valid_email),
        email_validator.validate(valid_email).is_ok(),
    );

    // 无效邮箱
    let invalid_email = "not-an-email";
    chain.add_step(
        "keycompute-auth",
        "validate_email_invalid",
        format!("拒绝无效邮箱: {}", invalid_email),
        email_validator.validate(invalid_email).is_err(),
    );

    // 2. 测试密码验证器
    let password_validator = PasswordValidator::new();

    // 有效密码
    let valid_password = "StrongP@ss123";
    chain.add_step(
        "keycompute-auth",
        "validate_password_valid",
        "验证强密码",
        password_validator.validate(valid_password).is_ok(),
    );

    // 无效密码
    let invalid_password = "weak";
    chain.add_step(
        "keycompute-auth",
        "validate_password_invalid",
        "拒绝弱密码",
        password_validator.validate(invalid_password).is_err(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试验证令牌生命周期
#[test]
fn test_verification_token_lifecycle() {
    let mut chain = VerificationChain::new();

    // 1. 创建验证记录
    let user_id = Uuid::new_v4();
    let verification = MockEmailVerification::new(user_id, "user@example.com", "token123");

    chain.add_step(
        "integration-tests",
        "verification::new",
        "验证记录创建",
        verification.token == "token123" && !verification.used,
    );

    // 2. 验证初始状态
    chain.add_step(
        "integration-tests",
        "verification::is_valid_initial",
        "初始状态有效",
        verification.is_valid(),
    );

    // 3. 模拟使用令牌
    let mut used_verification = verification.clone();
    used_verification.used = true;

    chain.add_step(
        "integration-tests",
        "verification::used",
        "使用后标记为已使用",
        used_verification.used,
    );

    chain.add_step(
        "integration-tests",
        "verification::is_valid_after_use",
        "使用后无效",
        !used_verification.is_valid(),
    );

    // 4. 模拟过期令牌
    let mut expired_verification =
        MockEmailVerification::new(user_id, "user@example.com", "token456");
    expired_verification.expires_at = chrono::Utc::now() - chrono::Duration::hours(1);

    chain.add_step(
        "integration-tests",
        "verification::expired",
        "过期令牌无效",
        !expired_verification.is_valid(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试邮件发送失败处理
#[tokio::test]
async fn test_email_send_failure_handling() {
    let mut chain = VerificationChain::new();
    let email_service = MockEmailService::new();

    // 1. 设置邮件服务失败模式
    email_service.set_should_fail(true);
    chain.add_step(
        "integration-tests",
        "set_failure_mode",
        "设置邮件服务失败模式",
        true,
    );

    // 2. 尝试发送验证邮件
    let result = email_service
        .send_verification_email("test@example.com", "token")
        .await;
    chain.add_step(
        "integration-tests",
        "send_verification_failure",
        "验证邮件发送失败",
        result.is_err(),
    );

    // 3. 验证没有邮件记录
    chain.add_step(
        "integration-tests",
        "no_email_recorded",
        "失败时不记录邮件",
        email_service.email_count() == 0,
    );

    // 4. 恢复服务并重新发送
    email_service.set_should_fail(false);
    let result = email_service
        .send_verification_email("test@example.com", "token")
        .await;
    chain.add_step(
        "integration-tests",
        "send_after_recovery",
        "恢复后发送成功",
        result.is_ok(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试重复注册检测
#[test]
fn test_duplicate_registration_detection() {
    let mut chain = VerificationChain::new();
    let db = MockUserTenantDatabase::new();

    // 1. 创建租户
    let tenant = db.create_test_tenant();

    // 2. 创建第一个用户
    let user1 = MockUser::new(tenant.id, "user@example.com", "user");
    db.insert_user(user1.clone());

    chain.add_step(
        "integration-tests",
        "create_first_user",
        "第一个用户创建成功",
        db.get_user(user1.id).is_some(),
    );

    // 3. 检查邮箱是否已存在
    let existing_user = db
        .get_users_by_tenant(tenant.id)
        .into_iter()
        .find(|u| u.email == "user@example.com");

    chain.add_step(
        "integration-tests",
        "detect_duplicate",
        "检测到邮箱已存在",
        existing_user.is_some(),
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试多租户注册隔离
#[test]
fn test_multi_tenant_registration_isolation() {
    let mut chain = VerificationChain::new();
    let db = MockUserTenantDatabase::new();

    // 1. 创建两个租户
    let tenant1 = MockTenant::new("Tenant A", "tenant-a");
    let tenant2 = MockTenant::new("Tenant B", "tenant-b");
    db.insert_tenant(tenant1.clone());
    db.insert_tenant(tenant2.clone());

    // 2. 在每个租户中创建相同邮箱的用户
    let user1 = MockUser::new(tenant1.id, "shared@example.com", "user");
    let user2 = MockUser::new(tenant2.id, "shared@example.com", "user");
    db.insert_user(user1.clone());
    db.insert_user(user2.clone());

    // 3. 验证用户属于不同租户
    let tenant1_users = db.get_users_by_tenant(tenant1.id);
    let tenant2_users = db.get_users_by_tenant(tenant2.id);

    chain.add_step(
        "integration-tests",
        "tenant1_has_user",
        "租户A有用户",
        tenant1_users
            .iter()
            .any(|u| u.email == "shared@example.com"),
    );

    chain.add_step(
        "integration-tests",
        "tenant2_has_user",
        "租户B有用户",
        tenant2_users
            .iter()
            .any(|u| u.email == "shared@example.com"),
    );

    // 4. 验证用户ID不同
    chain.add_step(
        "integration-tests",
        "different_user_ids",
        "不同租户用户ID不同",
        user1.id != user2.id,
    );

    chain.print_report();
    assert!(chain.all_passed());
}

/// 测试注册流程完整链路
#[tokio::test]
async fn test_registration_full_flow() {
    let mut chain = VerificationChain::new();
    let db = MockUserTenantDatabase::new();
    let email_service = MockEmailService::new();

    // 1. 创建租户
    let tenant = db.create_test_tenant();
    chain.add_step("integration-tests", "step1_create_tenant", "创建租户", true);

    // 2. 创建用户
    let user = db.create_test_user(tenant.id, "user");
    chain.add_step(
        "integration-tests",
        "step2_create_user",
        "创建用户",
        user.tenant_id == tenant.id,
    );

    // 3. 创建验证记录
    let verification = MockEmailVerification::new(user.id, &user.email, "verify_token_flow");
    chain.add_step(
        "integration-tests",
        "step3_create_verification",
        "创建验证记录",
        verification.is_valid(),
    );

    // 4. 发送验证邮件
    let result = email_service
        .send_verification_email(&user.email, &verification.token)
        .await;
    chain.add_step(
        "integration-tests",
        "step4_send_email",
        "发送验证邮件",
        result.is_ok(),
    );

    // 5. 验证邮件已发送
    chain.add_step(
        "integration-tests",
        "step5_verify_email_sent",
        "验证邮件已发送",
        email_service.has_verification_email(&user.email),
    );

    // 6. 模拟验证完成，发送欢迎邮件
    let welcome_result = email_service
        .send_welcome_email(&user.email, Some("TestUser"))
        .await;
    chain.add_step(
        "integration-tests",
        "step6_send_welcome",
        "发送欢迎邮件",
        welcome_result.is_ok(),
    );

    // 7. 验证所有邮件类型
    let all_emails = email_service.get_sent_emails();
    chain.add_step(
        "integration-tests",
        "step7_verify_all_emails",
        format!("总共发送 {} 封邮件", all_emails.len()),
        all_emails.len() == 2,
    );

    chain.print_report();
    assert!(chain.all_passed());
}
