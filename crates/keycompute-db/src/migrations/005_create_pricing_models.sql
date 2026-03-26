-- pricing_models: 模型定价表
CREATE TABLE pricing_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    model_name VARCHAR(100) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    input_price_per_1k DECIMAL(20, 10) NOT NULL,
    output_price_per_1k DECIMAL(20, 10) NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, model_name, provider, effective_from)
);

CREATE INDEX idx_pricing_models_tenant_id ON pricing_models(tenant_id);
CREATE INDEX idx_pricing_models_model ON pricing_models(model_name);
CREATE INDEX idx_pricing_models_provider ON pricing_models(provider);
CREATE INDEX idx_pricing_models_default ON pricing_models(is_default) WHERE is_default = TRUE;
