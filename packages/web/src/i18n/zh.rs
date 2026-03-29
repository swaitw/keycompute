use std::collections::HashMap;
use std::sync::LazyLock;

pub static ZH: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // ── 导航 ────────────────────────────────────
    m.insert("nav.home", "首页");
    m.insert("nav.usage", "用量统计");
    m.insert("nav.billing", "账单管理");
    m.insert("nav.api_keys", "API Keys");
    m.insert("nav.payments", "支付中心");
    m.insert("nav.payments.balance", "余额查询");
    m.insert("nav.payments.orders", "订单列表");
    m.insert("nav.payments.recharge", "充值");
    m.insert("nav.distribution", "分销中心");
    m.insert("nav.distribution.earnings", "分销收益");
    m.insert("nav.distribution.referrals", "推荐列表");
    m.insert("nav.distribution.invite", "邀请管理");
    m.insert("nav.user", "个人中心");
    m.insert("nav.user.profile", "个人资料");
    m.insert("nav.user.security", "安全设置");
    m.insert("nav.users", "用户管理");
    m.insert("nav.accounts", "账号管理");
    m.insert("nav.pricing", "定价管理");
    m.insert("nav.payment_orders", "支付订单");
    m.insert("nav.distribution_records", "分销记录");
    m.insert("nav.tenants", "租户管理");
    m.insert("nav.system", "系统诊断");
    m.insert("nav.settings", "系统设置");

    // ── 认证 ────────────────────────────────────
    m.insert("auth.login", "登录");
    m.insert("auth.register", "注册");
    m.insert("auth.logout", "退出登录");
    m.insert("auth.forgot_password", "忘记密码");
    m.insert("auth.reset_password", "重置密码");
    m.insert("auth.email", "邮箱");
    m.insert("auth.password", "密码");
    m.insert("auth.confirm_password", "确认密码");
    m.insert("auth.name", "姓名");
    m.insert("auth.remember_me", "记住我");
    m.insert("auth.no_account", "还没有账号？");
    m.insert("auth.has_account", "已有账号？");
    m.insert("auth.send_reset_email", "发送重置邮件");
    m.insert("auth.back_to_login", "返回登录");
    m.insert("auth.login_subtitle", "登录您的账户以继续");
    m.insert("auth.register_subtitle", "创建您的账户");
    m.insert("auth.reset_subtitle", "输入您的邮箱，我们将发送重置链接");
    m.insert("auth.reset_sent", "重置链接已发送到您的邮箱，请查收");
    m.insert("auth.register_now", "立即注册");
    m.insert("auth.login_now", "立即登录");
    m.insert("auth.email_placeholder", "请输入邮箱");
    m.insert("auth.password_placeholder", "请输入密码");
    m.insert("auth.name_placeholder", "请输入姓名");
    m.insert("auth.confirm_password_placeholder", "再次输入密码");
    m.insert("auth.reset_email_placeholder", "请输入注册邮箱");
    m.insert("auth.fill_all", "请填写邮箱和密码");
    m.insert("auth.fill_required", "请填写所有必填项");
    m.insert("auth.enter_email", "请输入邮箱地址");
    m.insert("auth.login_failed", "登录失败");
    m.insert("auth.register_failed", "注册失败");
    m.insert("auth.send_failed", "发送失败");
    m.insert("auth.sending", "发送中...");
    m.insert("auth.send_reset_link", "发送重置链接");
    m.insert("auth.logging_in", "登录中...");
    m.insert("auth.registering", "注册中...");
    m.insert("auth.password_min8", "密码至少8位");

    // ── 页面标题 ─────────────────────────────────
    m.insert("page.home", "仪表盘");
    m.insert("page.usage", "用量统计");
    m.insert("page.billing", "账单管理");
    m.insert("page.api_keys", "API Key 管理");
    m.insert("page.payments", "支付中心");
    m.insert("page.distribution", "分销中心");
    m.insert("page.profile", "个人资料");
    m.insert("page.security", "安全设置");
    m.insert("page.users", "用户管理");
    m.insert("page.accounts", "账号管理");
    m.insert("page.pricing", "定价管理");
    m.insert("page.payment_orders", "支付订单");
    m.insert("page.distribution_records", "分销记录");
    m.insert("page.tenants", "租户管理");
    m.insert("page.system", "系统诊断");
    m.insert("page.settings", "系统设置");
    m.insert("page.not_found", "页面不存在");

    // ── 表单 ────────────────────────────────────
    m.insert("form.save", "保存");
    m.insert("form.cancel", "取消");
    m.insert("form.confirm", "确认");
    m.insert("form.delete", "删除");
    m.insert("form.create", "新建");
    m.insert("form.edit", "编辑");
    m.insert("form.search", "搜索");
    m.insert("form.reset", "重置");
    m.insert("form.submit", "提交");
    m.insert("form.required", "此字段为必填项");
    m.insert("form.invalid_email", "请输入有效的邮箱地址");
    m.insert("form.password_too_short", "密码至少 8 位");
    m.insert("form.password_mismatch", "两次密码不一致");

    // ── 表格 ────────────────────────────────────
    m.insert("table.no_data", "暂无数据");
    m.insert("table.loading", "加载中...");
    m.insert("table.actions", "操作");
    m.insert("table.status", "状态");
    m.insert("table.created_at", "创建时间");
    m.insert("table.name", "名称");
    m.insert("table.email", "邮箱");
    m.insert("table.role", "角色");

    // ── 通用 ────────────────────────────────────
    m.insert("common.loading", "加载中");
    m.insert("common.error", "出错了");
    m.insert("common.success", "操作成功");
    m.insert("common.confirm_delete", "确定要删除吗？此操作不可撤销。");
    m.insert("common.copied", "已复制到剪贴板");
    m.insert("common.copy", "复制");
    m.insert("common.refresh", "刷新");
    m.insert("common.back", "返回");
    m.insert("common.yes", "是");
    m.insert("common.no", "否");
    m.insert("common.admin", "管理员");
    m.insert("common.user", "普通用户");
    m.insert("common.no_permission", "您没有权限访问此页面");
    m.insert("common.balance", "余额");
    m.insert("common.amount", "金额");
    m.insert("common.currency", "货币");
    m.insert("common.tokens", "Token 数");
    m.insert("common.requests", "请求数");
    m.insert("common.cost", "费用");
    m.insert("dashboard.greeting", "你好");
    m.insert("dashboard.subtitle", "这是您的控制台概览");
    m.insert("dashboard.api_calls", "API 调用次数");
    m.insert("dashboard.weekly_total", "本周累计");
    m.insert("dashboard.balance", "账户余额");
    m.insert("dashboard.available", "可用");
    m.insert("dashboard.active_keys", "活跃 Key");
    m.insert("dashboard.total", "总计");
    m.insert("dashboard.weekly_cost", "本周消耗");
    m.insert("dashboard.used", "已用");
    m.insert("dashboard.quick_links", "快速入口");
    m.insert("dashboard.manage_api_keys", "管理 API Key");
    m.insert("dashboard.recharge", "充値余额");
    m.insert("dashboard.account_settings", "账户设置");

    m
});
