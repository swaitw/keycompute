-- distribution_records: 二级分销记录
CREATE TABLE distribution_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    usage_log_id UUID NOT NULL REFERENCES usage_logs(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    beneficiary_id UUID NOT NULL,
    share_amount DECIMAL(20, 10) NOT NULL,
    share_ratio DECIMAL(5, 4) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    settled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_distribution_records_tenant_id ON distribution_records(tenant_id);
CREATE INDEX idx_distribution_records_usage_log_id ON distribution_records(usage_log_id);
CREATE INDEX idx_distribution_records_beneficiary_id ON distribution_records(beneficiary_id);
CREATE INDEX idx_distribution_records_status ON distribution_records(status);
