-- 0004 tenancy & organisation (DB design §9.3)

CREATE TABLE tenant_settings (
    tenant_id                BINARY(16)    NOT NULL,
    currency_decimals        TINYINT       NOT NULL DEFAULT 2,
    cash_rounding            DECIMAL(6,2)  NOT NULL DEFAULT 1.00,
    prices_include_tax       BOOLEAN       NOT NULL DEFAULT FALSE,
    is_vat_registered        BOOLEAN       NOT NULL DEFAULT FALSE,
    default_tax_rate_id      BINARY(16),
    allow_negative_stock     BOOLEAN       NOT NULL DEFAULT FALSE,
    costing_method           VARCHAR(20)   NOT NULL DEFAULT 'weighted_average',
    return_window_days       INT           NOT NULL DEFAULT 7,
    cashier_max_discount_pct DECIMAL(7,4)  NOT NULL DEFAULT 10,
    fiscal_year_start_month  TINYINT       NOT NULL DEFAULT 7,
    receipt_paper            VARCHAR(10)   NOT NULL DEFAULT '80mm',
    receipt_footer           VARCHAR(500),
    auto_print_receipt       BOOLEAN       NOT NULL DEFAULT TRUE,
    show_bengali_digits      BOOLEAN       NOT NULL DEFAULT FALSE,
    extra                    JSON          NOT NULL DEFAULT (JSON_OBJECT()),
    version                  INT           NOT NULL DEFAULT 1,
    updated_at               DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    updated_by               BINARY(16),
    PRIMARY KEY (tenant_id),
    CONSTRAINT fk_tenant_settings_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_ts_decimals  CHECK (currency_decimals BETWEEN 0 AND 4),
    CONSTRAINT ck_ts_rounding  CHECK (cash_rounding IN (0, 0.50, 1.00)),
    CONSTRAINT ck_ts_costing   CHECK (costing_method IN ('weighted_average','fifo')),
    CONSTRAINT ck_ts_return    CHECK (return_window_days >= 0),
    CONSTRAINT ck_ts_discount  CHECK (cashier_max_discount_pct BETWEEN 0 AND 100),
    CONSTRAINT ck_ts_fy        CHECK (fiscal_year_start_month BETWEEN 1 AND 12),
    CONSTRAINT ck_ts_paper     CHECK (receipt_paper IN ('58mm','80mm','a4'))
);

CREATE TABLE branches (
    tenant_id            BINARY(16)   NOT NULL,
    id                   BINARY(16)   NOT NULL,
    code                 VARCHAR(10)  NOT NULL,
    name                 VARCHAR(150) NOT NULL,
    address              VARCHAR(500),
    phone                VARCHAR(20),
    is_default           BOOLEAN      NOT NULL DEFAULT FALSE,
    default_key          TINYINT      AS (IF(is_default, 1, NULL)) VIRTUAL,
    status               VARCHAR(20)  NOT NULL DEFAULT 'active',
    default_location_id  BINARY(16),
    receipt_header       VARCHAR(500),
    version              INT          NOT NULL DEFAULT 1,
    created_at           DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at           DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by           BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_branches_code (tenant_id, code),
    UNIQUE KEY uq_branches_default (tenant_id, default_key),
    CONSTRAINT fk_branches_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_branches_code   CHECK (REGEXP_LIKE(code, '^[A-Z0-9]{1,10}$', 'c')),
    CONSTRAINT ck_branches_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE stock_locations (
    tenant_id     BINARY(16)   NOT NULL,
    id            BINARY(16)   NOT NULL,
    branch_id     BINARY(16)   NOT NULL,
    code          VARCHAR(20)  NOT NULL,
    name          VARCHAR(100) NOT NULL,
    location_type VARCHAR(20)  NOT NULL DEFAULT 'store',
    is_sellable   BOOLEAN      NOT NULL DEFAULT TRUE,
    status        VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_stock_locations_code (tenant_id, code),
    CONSTRAINT fk_stock_locations_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_stock_locations_branch FOREIGN KEY (tenant_id, branch_id) REFERENCES branches (tenant_id, id),
    CONSTRAINT ck_stock_locations_type   CHECK (location_type IN ('store','warehouse','damaged','quarantine','transit')),
    CONSTRAINT ck_stock_locations_status CHECK (status IN ('active','inactive'))
);

-- Circular reference: provisioning inserts the branch with default_location_id = NULL,
-- inserts the locations, then updates the branch (no deferrable constraints in MySQL).
ALTER TABLE branches ADD CONSTRAINT fk_branches_default_location
    FOREIGN KEY (tenant_id, default_location_id) REFERENCES stock_locations (tenant_id, id);

CREATE TABLE tenant_memberships (
    tenant_id     BINARY(16)   NOT NULL,
    id            BINARY(16)   NOT NULL,
    user_id       BINARY(16)   NOT NULL,
    status        VARCHAR(20)  NOT NULL DEFAULT 'active',
    is_owner      BOOLEAN      NOT NULL DEFAULT FALSE,
    perms_version INT          NOT NULL DEFAULT 1,
    display_name  VARCHAR(150),
    pin_hash      VARCHAR(255),
    joined_at     DATETIME(6),
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by    BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_memberships_user (tenant_id, user_id),
    KEY idx_memberships_user (user_id, status),
    CONSTRAINT fk_memberships_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_memberships_user   FOREIGN KEY (user_id)   REFERENCES users (id),
    CONSTRAINT ck_memberships_status CHECK (status IN ('invited','active','suspended','removed'))
);

CREATE TABLE invitations (
    tenant_id     BINARY(16)   NOT NULL,
    id            BINARY(16)   NOT NULL,
    destination   VARCHAR(254) NOT NULL,
    token_hash    BINARY(32)   NOT NULL,
    role_ids      JSON         NOT NULL,
    branch_ids    JSON,
    status        VARCHAR(20)  NOT NULL DEFAULT 'pending',
    expires_at    DATETIME(6)  NOT NULL,
    accepted_by   BINARY(16),
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_by    BINARY(16)   NOT NULL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_invitations_token (token_hash),
    CONSTRAINT fk_invitations_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_invitations_user   FOREIGN KEY (accepted_by) REFERENCES users (id),
    CONSTRAINT ck_invitations_status CHECK (status IN ('pending','accepted','revoked','expired'))
);

CREATE TABLE document_sequences (
    tenant_id       BINARY(16)  NOT NULL,
    id              BINARY(16)  NOT NULL,
    branch_id       BINARY(16),
    branch_key      BINARY(16)  AS (COALESCE(branch_id, 0x00000000000000000000000000000000)) VIRTUAL,
    doc_type        VARCHAR(30) NOT NULL,
    prefix          VARCHAR(10) NOT NULL,
    include_period  BOOLEAN     NOT NULL DEFAULT TRUE,
    reset_period    VARCHAR(10) NOT NULL DEFAULT 'never',
    period_key      VARCHAR(7),
    padding         TINYINT     NOT NULL DEFAULT 5,
    next_value      BIGINT      NOT NULL DEFAULT 1,
    updated_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_document_sequences (tenant_id, branch_key, doc_type),
    CONSTRAINT fk_docseq_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_docseq_branch FOREIGN KEY (tenant_id, branch_id) REFERENCES branches (tenant_id, id),
    CONSTRAINT ck_docseq_type   CHECK (doc_type IN ('sale','sale_return','purchase','purchase_return',
                         'goods_receipt','payment_in','payment_out','expense','income','adjustment',
                         'transfer','count','journal','customer','supplier','product')),
    CONSTRAINT ck_docseq_reset  CHECK (reset_period IN ('never','yearly','monthly')),
    CONSTRAINT ck_docseq_pad    CHECK (padding BETWEEN 3 AND 10),
    CONSTRAINT ck_docseq_next   CHECK (next_value >= 1)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- tenant_settings
CREATE TRIGGER trg_tenant_settings_tg_ins BEFORE INSERT ON tenant_settings FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tenant_settings_tg_upd BEFORE UPDATE ON tenant_settings FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tenant_settings_tg_del BEFORE DELETE ON tenant_settings FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- branches
CREATE TRIGGER trg_branches_tg_ins BEFORE INSERT ON branches FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_branches_tg_upd BEFORE UPDATE ON branches FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_branches_tg_del BEFORE DELETE ON branches FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_locations
CREATE TRIGGER trg_stock_locations_tg_ins BEFORE INSERT ON stock_locations FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_locations_tg_upd BEFORE UPDATE ON stock_locations FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_locations_tg_del BEFORE DELETE ON stock_locations FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- tenant_memberships
CREATE TRIGGER trg_tenant_memberships_tg_ins BEFORE INSERT ON tenant_memberships FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tenant_memberships_tg_upd BEFORE UPDATE ON tenant_memberships FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tenant_memberships_tg_del BEFORE DELETE ON tenant_memberships FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- invitations
CREATE TRIGGER trg_invitations_tg_ins BEFORE INSERT ON invitations FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_invitations_tg_upd BEFORE UPDATE ON invitations FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_invitations_tg_del BEFORE DELETE ON invitations FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- document_sequences
CREATE TRIGGER trg_document_sequences_tg_ins BEFORE INSERT ON document_sequences FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_document_sequences_tg_upd BEFORE UPDATE ON document_sequences FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_document_sequences_tg_del BEFORE DELETE ON document_sequences FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
