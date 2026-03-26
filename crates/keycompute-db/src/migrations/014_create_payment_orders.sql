-- 支付订单表
-- 用于存储用户充值订单记录

CREATE TABLE IF NOT EXISTS payment_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 租户ID
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    -- 用户ID
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- 支付宝订单号（外部订单号）
    out_trade_no VARCHAR(64) NOT NULL UNIQUE,
    -- 支付宝交易号（支付宝返回）
    trade_no VARCHAR(64),
    -- 订单金额（单位：元）
    amount DECIMAL(12, 2) NOT NULL,
    -- 币种（默认CNY）
    currency VARCHAR(8) NOT NULL DEFAULT 'CNY',
    -- 订单状态: pending/paid/failed/closed/refunded
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- 支付方式: alipay
    payment_method VARCHAR(20) NOT NULL DEFAULT 'alipay',
    -- 商品标题
    subject VARCHAR(256) NOT NULL,
    -- 商品描述
    body TEXT,
    -- 支付时间
    paid_at TIMESTAMP WITH TIME ZONE,
    -- 关闭时间
    closed_at TIMESTAMP WITH TIME ZONE,
    -- 过期时间
    expired_at TIMESTAMP WITH TIME ZONE NOT NULL,
    -- 支付URL（用于前端跳转）
    pay_url TEXT,
    -- 回调通知原始数据
    notify_data JSONB,
    -- 备注信息
    remarks TEXT,
    -- 创建时间
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    -- 更新时间
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_payment_orders_tenant_id ON payment_orders(tenant_id);
CREATE INDEX idx_payment_orders_user_id ON payment_orders(user_id);
CREATE INDEX idx_payment_orders_out_trade_no ON payment_orders(out_trade_no);
CREATE INDEX idx_payment_orders_trade_no ON payment_orders(trade_no);
CREATE INDEX idx_payment_orders_status ON payment_orders(status);
CREATE INDEX idx_payment_orders_created_at ON payment_orders(created_at);

-- 添加注释
COMMENT ON TABLE payment_orders IS '支付订单表';
COMMENT ON COLUMN payment_orders.id IS '订单ID';
COMMENT ON COLUMN payment_orders.tenant_id IS '租户ID';
COMMENT ON COLUMN payment_orders.user_id IS '用户ID';
COMMENT ON COLUMN payment_orders.out_trade_no IS '商户订单号（外部订单号）';
COMMENT ON COLUMN payment_orders.trade_no IS '支付宝交易号';
COMMENT ON COLUMN payment_orders.amount IS '订单金额（单位：元）';
COMMENT ON COLUMN payment_orders.currency IS '币种';
COMMENT ON COLUMN payment_orders.status IS '订单状态: pending/paid/failed/closed/refunded';
COMMENT ON COLUMN payment_orders.payment_method IS '支付方式';
COMMENT ON COLUMN payment_orders.subject IS '商品标题';
COMMENT ON COLUMN payment_orders.body IS '商品描述';
COMMENT ON COLUMN payment_orders.paid_at IS '支付时间';
COMMENT ON COLUMN payment_orders.closed_at IS '关闭时间';
COMMENT ON COLUMN payment_orders.expired_at IS '过期时间';
COMMENT ON COLUMN payment_orders.pay_url IS '支付URL';
COMMENT ON COLUMN payment_orders.notify_data IS '回调通知原始数据';
COMMENT ON COLUMN payment_orders.remarks IS '备注信息';

-- 用户余额表
CREATE TABLE IF NOT EXISTS user_balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 租户ID
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    -- 用户ID
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    -- 可用余额（单位：元）
    available_balance DECIMAL(12, 4) NOT NULL DEFAULT 0,
    -- 冻结余额（单位：元）
    frozen_balance DECIMAL(12, 4) NOT NULL DEFAULT 0,
    -- 累计充值金额
    total_recharged DECIMAL(12, 4) NOT NULL DEFAULT 0,
    -- 累计消费金额
    total_consumed DECIMAL(12, 4) NOT NULL DEFAULT 0,
    -- 创建时间
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    -- 更新时间
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_user_balances_tenant_id ON user_balances(tenant_id);
CREATE INDEX idx_user_balances_user_id ON user_balances(user_id);

-- 添加注释
COMMENT ON TABLE user_balances IS '用户余额表';
COMMENT ON COLUMN user_balances.available_balance IS '可用余额（单位：元）';
COMMENT ON COLUMN user_balances.frozen_balance IS '冻结余额（单位：元）';
COMMENT ON COLUMN user_balances.total_recharged IS '累计充值金额';
COMMENT ON COLUMN user_balances.total_consumed IS '累计消费金额';

-- 余额变动记录表
CREATE TABLE IF NOT EXISTS balance_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 租户ID
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    -- 用户ID
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- 关联订单ID（可选）
    order_id UUID REFERENCES payment_orders(id),
    -- 关联使用日志ID（可选）
    usage_log_id UUID REFERENCES usage_logs(id),
    -- 交易类型: recharge/consume/refund/freeze/unfreeze
    transaction_type VARCHAR(20) NOT NULL,
    -- 变动金额（正数为增加，负数为减少）
    amount DECIMAL(12, 4) NOT NULL,
    -- 变动前余额
    balance_before DECIMAL(12, 4) NOT NULL,
    -- 变动后余额
    balance_after DECIMAL(12, 4) NOT NULL,
    -- 币种
    currency VARCHAR(8) NOT NULL DEFAULT 'CNY',
    -- 备注
    description TEXT,
    -- 创建时间
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_balance_transactions_tenant_id ON balance_transactions(tenant_id);
CREATE INDEX idx_balance_transactions_user_id ON balance_transactions(user_id);
CREATE INDEX idx_balance_transactions_order_id ON balance_transactions(order_id);
CREATE INDEX idx_balance_transactions_usage_log_id ON balance_transactions(usage_log_id);
CREATE INDEX idx_balance_transactions_type ON balance_transactions(transaction_type);
CREATE INDEX idx_balance_transactions_created_at ON balance_transactions(created_at);

-- 添加注释
COMMENT ON TABLE balance_transactions IS '余额变动记录表';
COMMENT ON COLUMN balance_transactions.transaction_type IS '交易类型: recharge/consume/refund/freeze/unfreeze';
COMMENT ON COLUMN balance_transactions.amount IS '变动金额（正数为增加，负数为减少）';
COMMENT ON COLUMN balance_transactions.balance_before IS '变动前余额';
COMMENT ON COLUMN balance_transactions.balance_after IS '变动后余额';
