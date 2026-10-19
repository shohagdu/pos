-- 0002 platform core: tenants, plans, features (DB design §9.1)

CREATE TABLE tenants (
    id                BINARY(16)   NOT NULL,
    slug              VARCHAR(63)  NOT NULL,
    name              VARCHAR(200) NOT NULL,
    legal_name        VARCHAR(200),
    business_type     VARCHAR(20)  NOT NULL DEFAULT 'retail',
    status            VARCHAR(20)  NOT NULL DEFAULT 'provisioning',
    country_code      CHAR(2)      NOT NULL DEFAULT 'BD',
    currency_code     CHAR(3)      NOT NULL DEFAULT 'BDT',
    timezone          VARCHAR(64)  NOT NULL DEFAULT 'Asia/Dhaka',
    locale            VARCHAR(10)  NOT NULL DEFAULT 'en',
    phone             VARCHAR(20),
    email             VARCHAR(254),
    address           VARCHAR(500),
    district          VARCHAR(100),
    bin_number        VARCHAR(30),
    trade_license_no  VARCHAR(50),
    logo_file_id      BINARY(16),
    is_sample_data    BOOLEAN      NOT NULL DEFAULT FALSE,
    suspended_reason  VARCHAR(500),
    terminated_at     DATETIME(6),
    created_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_tenants_slug (slug),
    CONSTRAINT ck_tenants_slug   CHECK (REGEXP_LIKE(slug, '^[a-z0-9][a-z0-9-]{2,62}$', 'c')),
    CONSTRAINT ck_tenants_name   CHECK (CHAR_LENGTH(name) >= 2),
    CONSTRAINT ck_tenants_type   CHECK (business_type IN ('retail','grocery','fashion','electronics',
                                                          'wholesale','pharmacy','restaurant','other')),
    CONSTRAINT ck_tenants_status CHECK (status IN ('provisioning','active','suspended','terminated'))
);

CREATE TABLE plans (
    id            BINARY(16)   NOT NULL,
    code          VARCHAR(40)  NOT NULL,
    name          VARCHAR(100) NOT NULL,
    description   TEXT,
    is_public     BOOLEAN      NOT NULL DEFAULT TRUE,
    is_trial      BOOLEAN      NOT NULL DEFAULT FALSE,
    trial_days    INT,
    sort_order    INT          NOT NULL DEFAULT 0,
    status        VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_plans_code (code),
    CONSTRAINT ck_plans_trial  CHECK (trial_days IS NULL OR trial_days > 0),
    CONSTRAINT ck_plans_status CHECK (status IN ('active','retired'))
);

CREATE TABLE plan_prices (
    id             BINARY(16)    NOT NULL,
    plan_id        BINARY(16)    NOT NULL,
    billing_cycle  VARCHAR(20)   NOT NULL,
    currency_code  CHAR(3)       NOT NULL DEFAULT 'BDT',
    amount         DECIMAL(18,2) NOT NULL,
    is_active      BOOLEAN       NOT NULL DEFAULT TRUE,
    created_at     DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_plan_prices (plan_id, billing_cycle, currency_code),
    CONSTRAINT fk_plan_prices_plan FOREIGN KEY (plan_id) REFERENCES plans (id),
    CONSTRAINT ck_plan_prices_cycle  CHECK (billing_cycle IN ('monthly','quarterly','annual')),
    CONSTRAINT ck_plan_prices_amount CHECK (amount >= 0)
);

CREATE TABLE features (
    code         VARCHAR(60)  NOT NULL,
    name         VARCHAR(100) NOT NULL,
    kind         VARCHAR(10)  NOT NULL,
    description  TEXT,
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (code),
    CONSTRAINT ck_features_kind CHECK (kind IN ('flag','limit'))
);

CREATE TABLE plan_features (
    plan_id       BINARY(16)  NOT NULL,
    feature_code  VARCHAR(60) NOT NULL,
    enabled       BOOLEAN     NOT NULL DEFAULT TRUE,
    limit_value   BIGINT,
    PRIMARY KEY (plan_id, feature_code),
    CONSTRAINT fk_plan_features_plan    FOREIGN KEY (plan_id) REFERENCES plans (id) ON DELETE CASCADE,
    CONSTRAINT fk_plan_features_feature FOREIGN KEY (feature_code) REFERENCES features (code),
    CONSTRAINT ck_plan_features_limit   CHECK (limit_value IS NULL OR limit_value >= 0)
);

CREATE TABLE platform_settings (
    setting_key  VARCHAR(100) NOT NULL,
    value        JSON         NOT NULL,
    updated_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    updated_by   BINARY(16),
    PRIMARY KEY (setting_key)
);

CREATE TABLE platform_sequences (
    name        VARCHAR(50) NOT NULL,
    next_value  BIGINT      NOT NULL DEFAULT 1,
    PRIMARY KEY (name)
);

-- ---------------------------------------------------------------------------
-- Seeds (Requirement Analysis §11 — illustrative prices, business decision)
-- ---------------------------------------------------------------------------
INSERT INTO features (code, name, kind, description) VALUES
 ('branches.max',            'Branches',                'limit', 'Maximum active branches'),
 ('users.max',               'Users',                   'limit', 'Maximum active members'),
 ('products.max',            'Products',                'limit', 'Maximum active product variants'),
 ('registers.max',           'Cash registers',          'limit', 'Maximum cash registers (R2)'),
 ('invoices.monthly_max',    'Invoices per month',      'limit', 'Maximum sales invoices per calendar month'),
 ('storage.mb',              'File storage (MB)',       'limit', 'Object storage quota'),
 ('pos.core',                'POS',                     'flag',  'Point of sale'),
 ('inventory.adjustments',   'Stock adjustments',       'flag',  'Stock adjustment documents'),
 ('purchases.orders',        'Purchase orders',         'flag',  'Purchase orders and approval (R2)'),
 ('inventory.transfers',     'Stock transfers',         'flag',  'Transfers between locations (R2)'),
 ('products.variants',       'Product variants',        'flag',  'Variant matrix UI (R2)'),
 ('accounting.double_entry', 'Double-entry accounting', 'flag',  'Chart of accounts and journals (R2)'),
 ('reports.export',          'Report export',           'flag',  'CSV/XLSX/PDF export'),
 ('reports.advanced',        'Advanced reports',        'flag',  'Aging, VAT, movers (R2)'),
 ('notifications.sms',       'SMS notifications',       'flag',  'SMS channel (R2)'),
 ('api.access',              'Public API',              'flag',  'API keys and webhooks (R3)'),
 ('whitelabel',              'White label',             'flag',  'Custom branding and domain (R4)');

SET @plan_trial      = UUID_TO_BIN('019a0000-0000-7000-8000-000000000001'),
    @plan_starter    = UUID_TO_BIN('019a0000-0000-7000-8000-000000000002'),
    @plan_business   = UUID_TO_BIN('019a0000-0000-7000-8000-000000000003'),
    @plan_enterprise = UUID_TO_BIN('019a0000-0000-7000-8000-000000000004');

INSERT INTO plans (id, code, name, is_public, is_trial, trial_days, sort_order) VALUES
 (@plan_trial,      'trial',      'Free Trial', FALSE, TRUE, 14,   0),
 (@plan_starter,    'starter',    'Starter',    TRUE,  FALSE, NULL, 1),
 (@plan_business,   'business',   'Business',   TRUE,  FALSE, NULL, 2),
 (@plan_enterprise, 'enterprise', 'Enterprise', FALSE, FALSE, NULL, 3);

INSERT INTO plan_prices (id, plan_id, billing_cycle, amount) VALUES
 (UUID_TO_BIN('019a0000-0000-7000-8001-000000000001'), @plan_starter,  'monthly',     990.00),
 (UUID_TO_BIN('019a0000-0000-7000-8001-000000000002'), @plan_starter,  'quarterly',  2821.50),
 (UUID_TO_BIN('019a0000-0000-7000-8001-000000000003'), @plan_starter,  'annual',    10098.00),
 (UUID_TO_BIN('019a0000-0000-7000-8001-000000000004'), @plan_business, 'monthly',    2490.00),
 (UUID_TO_BIN('019a0000-0000-7000-8001-000000000005'), @plan_business, 'quarterly',  7096.50),
 (UUID_TO_BIN('019a0000-0000-7000-8001-000000000006'), @plan_business, 'annual',    25398.00);

-- limits: NULL = unlimited
INSERT INTO plan_features (plan_id, feature_code, enabled, limit_value) VALUES
 (@plan_trial, 'branches.max', TRUE, 1), (@plan_trial, 'users.max', TRUE, 3), (@plan_trial, 'products.max', TRUE, 500),
 (@plan_trial, 'registers.max', TRUE, 1), (@plan_trial, 'invoices.monthly_max', TRUE, 1000), (@plan_trial, 'storage.mb', TRUE, 200),
 (@plan_trial, 'pos.core', TRUE, NULL), (@plan_trial, 'inventory.adjustments', TRUE, NULL), (@plan_trial, 'purchases.orders', TRUE, NULL),
 (@plan_trial, 'inventory.transfers', FALSE, NULL), (@plan_trial, 'products.variants', TRUE, NULL), (@plan_trial, 'accounting.double_entry', TRUE, NULL),
 (@plan_trial, 'reports.export', TRUE, NULL), (@plan_trial, 'reports.advanced', TRUE, NULL), (@plan_trial, 'notifications.sms', FALSE, NULL),
 (@plan_trial, 'api.access', FALSE, NULL), (@plan_trial, 'whitelabel', FALSE, NULL),

 (@plan_starter, 'branches.max', TRUE, 1), (@plan_starter, 'users.max', TRUE, 3), (@plan_starter, 'products.max', TRUE, 2000),
 (@plan_starter, 'registers.max', TRUE, 1), (@plan_starter, 'invoices.monthly_max', TRUE, 5000), (@plan_starter, 'storage.mb', TRUE, 500),
 (@plan_starter, 'pos.core', TRUE, NULL), (@plan_starter, 'inventory.adjustments', TRUE, NULL), (@plan_starter, 'purchases.orders', FALSE, NULL),
 (@plan_starter, 'inventory.transfers', FALSE, NULL), (@plan_starter, 'products.variants', TRUE, NULL), (@plan_starter, 'accounting.double_entry', FALSE, NULL),
 (@plan_starter, 'reports.export', TRUE, NULL), (@plan_starter, 'reports.advanced', FALSE, NULL), (@plan_starter, 'notifications.sms', FALSE, NULL),
 (@plan_starter, 'api.access', FALSE, NULL), (@plan_starter, 'whitelabel', FALSE, NULL),

 (@plan_business, 'branches.max', TRUE, 3), (@plan_business, 'users.max', TRUE, 15), (@plan_business, 'products.max', TRUE, NULL),
 (@plan_business, 'registers.max', TRUE, 5), (@plan_business, 'invoices.monthly_max', TRUE, NULL), (@plan_business, 'storage.mb', TRUE, 5000),
 (@plan_business, 'pos.core', TRUE, NULL), (@plan_business, 'inventory.adjustments', TRUE, NULL), (@plan_business, 'purchases.orders', TRUE, NULL),
 (@plan_business, 'inventory.transfers', TRUE, NULL), (@plan_business, 'products.variants', TRUE, NULL), (@plan_business, 'accounting.double_entry', TRUE, NULL),
 (@plan_business, 'reports.export', TRUE, NULL), (@plan_business, 'reports.advanced', TRUE, NULL), (@plan_business, 'notifications.sms', TRUE, NULL),
 (@plan_business, 'api.access', FALSE, NULL), (@plan_business, 'whitelabel', FALSE, NULL),

 (@plan_enterprise, 'branches.max', TRUE, NULL), (@plan_enterprise, 'users.max', TRUE, NULL), (@plan_enterprise, 'products.max', TRUE, NULL),
 (@plan_enterprise, 'registers.max', TRUE, NULL), (@plan_enterprise, 'invoices.monthly_max', TRUE, NULL), (@plan_enterprise, 'storage.mb', TRUE, NULL),
 (@plan_enterprise, 'pos.core', TRUE, NULL), (@plan_enterprise, 'inventory.adjustments', TRUE, NULL), (@plan_enterprise, 'purchases.orders', TRUE, NULL),
 (@plan_enterprise, 'inventory.transfers', TRUE, NULL), (@plan_enterprise, 'products.variants', TRUE, NULL), (@plan_enterprise, 'accounting.double_entry', TRUE, NULL),
 (@plan_enterprise, 'reports.export', TRUE, NULL), (@plan_enterprise, 'reports.advanced', TRUE, NULL), (@plan_enterprise, 'notifications.sms', TRUE, NULL),
 (@plan_enterprise, 'api.access', TRUE, NULL), (@plan_enterprise, 'whitelabel', TRUE, NULL);

-- Reserved platform tenant: owns platform-level files (DB design §13.1)
INSERT INTO tenants (id, slug, name, business_type, status)
VALUES (UUID_TO_BIN('00000000-0000-7000-8000-000000000000'), 'platform', 'Platform', 'other', 'active');

INSERT INTO platform_sequences (name, next_value) VALUES ('subscription_invoice', 1);
