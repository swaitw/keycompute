use std::collections::HashMap;
use std::sync::LazyLock;

pub static EN: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // ── Navigation ──────────────────────────────
    m.insert("nav.home", "Home");
    m.insert("nav.usage", "Usage");
    m.insert("nav.billing", "Billing");
    m.insert("nav.api_keys", "API Keys");
    m.insert("nav.payments", "Payments");
    m.insert("nav.payments.balance", "Balance");
    m.insert("nav.payments.orders", "Orders");
    m.insert("nav.payments.recharge", "Recharge");
    m.insert("nav.distribution", "Distribution");
    m.insert("nav.distribution.earnings", "Earnings");
    m.insert("nav.distribution.referrals", "Referrals");
    m.insert("nav.distribution.invite", "Invite");
    m.insert("nav.user", "Profile");
    m.insert("nav.user.profile", "My Profile");
    m.insert("nav.user.security", "Security");
    m.insert("nav.users", "Users");
    m.insert("nav.accounts", "Accounts");
    m.insert("nav.pricing", "Pricing");
    m.insert("nav.payment_orders", "Payment Orders");
    m.insert("nav.distribution_records", "Distribution Records");
    m.insert("nav.tenants", "Tenants");
    m.insert("nav.system", "System");
    m.insert("nav.settings", "Settings");

    // ── Auth ────────────────────────────────────
    m.insert("auth.login", "Sign In");
    m.insert("auth.register", "Sign Up");
    m.insert("auth.logout", "Sign Out");
    m.insert("auth.forgot_password", "Forgot Password");
    m.insert("auth.reset_password", "Reset Password");
    m.insert("auth.email", "Email");
    m.insert("auth.password", "Password");
    m.insert("auth.confirm_password", "Confirm Password");
    m.insert("auth.name", "Name");
    m.insert("auth.remember_me", "Remember me");
    m.insert("auth.no_account", "Don't have an account?");
    m.insert("auth.has_account", "Already have an account?");
    m.insert("auth.send_reset_email", "Send Reset Email");
    m.insert("auth.back_to_login", "Back to Sign In");
    m.insert("auth.login_subtitle", "Sign in to your account to continue");
    m.insert("auth.register_subtitle", "Create your account");
    m.insert(
        "auth.reset_subtitle",
        "Enter your email and we'll send you a reset link",
    );
    m.insert("auth.reset_sent", "Reset link has been sent to your email");
    m.insert("auth.register_now", "Sign up");
    m.insert("auth.login_now", "Sign in");
    m.insert("auth.email_placeholder", "Enter your email");
    m.insert("auth.password_placeholder", "Enter your password");
    m.insert("auth.name_placeholder", "Enter your name");
    m.insert("auth.confirm_password_placeholder", "Re-enter password");
    m.insert(
        "auth.reset_email_placeholder",
        "Enter your registered email",
    );
    m.insert("auth.fill_all", "Please fill in email and password");
    m.insert("auth.fill_required", "Please fill in all required fields");
    m.insert("auth.enter_email", "Please enter your email address");
    m.insert("auth.login_failed", "Login failed");
    m.insert("auth.register_failed", "Registration failed");
    m.insert("auth.send_failed", "Failed to send");
    m.insert("auth.sending", "Sending...");
    m.insert("auth.send_reset_link", "Send Reset Link");
    m.insert("auth.logging_in", "Signing in...");
    m.insert("auth.registering", "Signing up...");
    m.insert(
        "auth.password_min8",
        "Password must be at least 8 characters",
    );

    // ── Page Titles ──────────────────────────────
    m.insert("page.home", "Dashboard");
    m.insert("page.usage", "Usage");
    m.insert("page.billing", "Billing");
    m.insert("page.api_keys", "API Keys");
    m.insert("page.payments", "Payments");
    m.insert("page.distribution", "Distribution");
    m.insert("page.profile", "Profile");
    m.insert("page.security", "Security");
    m.insert("page.users", "User Management");
    m.insert("page.accounts", "Account Management");
    m.insert("page.pricing", "Pricing");
    m.insert("page.payment_orders", "Payment Orders");
    m.insert("page.distribution_records", "Distribution Records");
    m.insert("page.tenants", "Tenants");
    m.insert("page.system", "System Diagnostics");
    m.insert("page.settings", "Settings");
    m.insert("page.not_found", "Page Not Found");

    // ── Form ────────────────────────────────────
    m.insert("form.save", "Save");
    m.insert("form.cancel", "Cancel");
    m.insert("form.confirm", "Confirm");
    m.insert("form.delete", "Delete");
    m.insert("form.create", "Create");
    m.insert("form.edit", "Edit");
    m.insert("form.search", "Search");
    m.insert("form.reset", "Reset");
    m.insert("form.submit", "Submit");
    m.insert("form.required", "This field is required");
    m.insert("form.invalid_email", "Please enter a valid email");
    m.insert(
        "form.password_too_short",
        "Password must be at least 8 characters",
    );
    m.insert("form.password_mismatch", "Passwords do not match");

    // ── Table ───────────────────────────────────
    m.insert("table.no_data", "No data");
    m.insert("table.loading", "Loading...");
    m.insert("table.actions", "Actions");
    m.insert("table.status", "Status");
    m.insert("table.created_at", "Created At");
    m.insert("table.name", "Name");
    m.insert("table.email", "Email");
    m.insert("table.role", "Role");

    // ── Common ──────────────────────────────────
    m.insert("common.loading", "Loading");
    m.insert("common.error", "Something went wrong");
    m.insert("common.success", "Success");
    m.insert(
        "common.confirm_delete",
        "Are you sure? This action cannot be undone.",
    );
    m.insert("common.copied", "Copied to clipboard");
    m.insert("common.copy", "Copy");
    m.insert("common.refresh", "Refresh");
    m.insert("common.back", "Back");
    m.insert("common.yes", "Yes");
    m.insert("common.no", "No");
    m.insert("common.admin", "Admin");
    m.insert("common.user", "User");
    m.insert(
        "common.no_permission",
        "You don't have permission to view this page",
    );
    m.insert("common.balance", "Balance");
    m.insert("common.amount", "Amount");
    m.insert("common.currency", "Currency");
    m.insert("common.tokens", "Tokens");
    m.insert("common.requests", "Requests");
    m.insert("common.cost", "Cost");
    m.insert("dashboard.greeting", "Hello");
    m.insert("dashboard.subtitle", "Here is your console overview");
    m.insert("dashboard.api_calls", "API Calls");
    m.insert("dashboard.weekly_total", "This Week");
    m.insert("dashboard.balance", "Account Balance");
    m.insert("dashboard.available", "Available");
    m.insert("dashboard.active_keys", "Active Keys");
    m.insert("dashboard.total", "Total");
    m.insert("dashboard.weekly_cost", "Weekly Cost");
    m.insert("dashboard.used", "Used");
    m.insert("dashboard.quick_links", "Quick Links");
    m.insert("dashboard.manage_api_keys", "Manage API Keys");
    m.insert("dashboard.recharge", "Recharge Balance");
    m.insert("dashboard.account_settings", "Account Settings");

    m
});
