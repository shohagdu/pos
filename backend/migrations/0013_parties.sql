-- 0013 parties: customers, suppliers and the append-only party ledger
-- (DB design §9.11). Customer groups are Release 2. Every business has one
-- walk-in customer that never carries a balance (PTY-R03).

CREATE TABLE customers (
    tenant_id          BINARY(16)    NOT NULL,
    id                 BINARY(16)    NOT NULL,
    code               VARCHAR(20)   NOT NULL,
    name               VARCHAR(150)  NOT NULL,
    phone              VARCHAR(20),
    email              VARCHAR(254),
    address            VARCHAR(500),
    customer_group_id  BINARY(16),
    credit_limit       DECIMAL(18,2),
    opening_balance    DECIMAL(18,2) NOT NULL DEFAULT 0,
    current_balance    DECIMAL(18,2) NOT NULL DEFAULT 0,
    is_walk_in         BOOLEAN       NOT NULL DEFAULT FALSE,
    notes              TEXT,
    status             VARCHAR(20)   NOT NULL DEFAULT 'active',
    is_sample          BOOLEAN       NOT NULL DEFAULT FALSE,
    version            INT           NOT NULL DEFAULT 1,
    created_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by         BINARY(16),
    deleted_at         DATETIME(6),
    live_phone         VARCHAR(20)   AS (IF(deleted_at IS NULL, phone, NULL)) VIRTUAL,
    walk_in_key        TINYINT       AS (IF(is_walk_in, 1, NULL)) VIRTUAL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_customers_code (tenant_id, code),
    UNIQUE KEY uq_customers_phone (tenant_id, live_phone),
    UNIQUE KEY uq_customers_walk_in (tenant_id, walk_in_key),
    KEY idx_customers_name (tenant_id, name),
    KEY idx_customers_due (tenant_id, current_balance),
    CONSTRAINT fk_customers_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_customers_credit  CHECK (credit_limit IS NULL OR credit_limit >= 0),
    CONSTRAINT ck_customers_walk_in CHECK (NOT is_walk_in OR (current_balance = 0 AND opening_balance = 0)),
    CONSTRAINT ck_customers_status  CHECK (status IN ('active','inactive'))
);

CREATE TABLE suppliers (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    code             VARCHAR(20)   NOT NULL,
    name             VARCHAR(150)  NOT NULL,
    contact_person   VARCHAR(150),
    phone            VARCHAR(20),
    email            VARCHAR(254),
    address          VARCHAR(500),
    opening_balance  DECIMAL(18,2) NOT NULL DEFAULT 0,
    current_balance  DECIMAL(18,2) NOT NULL DEFAULT 0,
    notes            TEXT,
    status           VARCHAR(20)   NOT NULL DEFAULT 'active',
    version          INT           NOT NULL DEFAULT 1,
    created_at       DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at       DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by       BINARY(16),
    deleted_at       DATETIME(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_suppliers_code (tenant_id, code),
    KEY idx_suppliers_name (tenant_id, name),
    KEY idx_suppliers_due (tenant_id, current_balance),
    CONSTRAINT fk_suppliers_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_suppliers_status CHECK (status IN ('active','inactive'))
);

-- amount > 0 increases what the customer owes us / what we owe the supplier.
CREATE TABLE party_ledger_entries (
    tenant_id       BINARY(16)    NOT NULL,
    id              BINARY(16)    NOT NULL,
    party_type      VARCHAR(10)   NOT NULL,
    party_id        BINARY(16)    NOT NULL,
    branch_id       BINARY(16),
    entry_type      VARCHAR(20)   NOT NULL,
    amount          DECIMAL(18,2) NOT NULL,
    balance_after   DECIMAL(18,2) NOT NULL,
    business_date   DATE          NOT NULL,
    reference_type  VARCHAR(30)   NOT NULL,
    reference_id    BINARY(16)    NOT NULL,
    reference_no    VARCHAR(40),
    note            VARCHAR(500),
    created_by      BINARY(16),
    created_at      DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_party_ledger_party (tenant_id, party_type, party_id, business_date, created_at),
    KEY idx_party_ledger_ref (tenant_id, reference_type, reference_id),
    CONSTRAINT fk_party_ledger_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_party_ledger_party  CHECK (party_type IN ('customer','supplier')),
    CONSTRAINT ck_party_ledger_type   CHECK (entry_type IN ('opening','sale','sale_cancel','sale_return',
                        'purchase','purchase_cancel','purchase_return','payment_in','payment_out',
                        'payment_void','refund_out','refund_in','adjustment')),
    CONSTRAINT ck_party_ledger_amount CHECK (amount <> 0)
);

-- Walk-in customer for businesses created before this migration.
SET @app_platform = 1;

INSERT INTO customers (tenant_id, id, code, name, is_walk_in)
SELECT t.id, UUID_TO_BIN(UUID(), 1), 'WALKIN', 'Walk-in Customer', TRUE FROM tenants t WHERE t.slug <> 'platform';

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- customers
CREATE TRIGGER trg_customers_tg_ins BEFORE INSERT ON customers FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_customers_tg_upd BEFORE UPDATE ON customers FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_customers_tg_del BEFORE DELETE ON customers FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- suppliers
CREATE TRIGGER trg_suppliers_tg_ins BEFORE INSERT ON suppliers FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_suppliers_tg_upd BEFORE UPDATE ON suppliers FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_suppliers_tg_del BEFORE DELETE ON suppliers FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- party_ledger_entries
CREATE TRIGGER trg_party_ledger_entries_tg_ins BEFORE INSERT ON party_ledger_entries FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_party_ledger_entries_ao_upd BEFORE UPDATE ON party_ledger_entries FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_party_ledger_entries_ao_del BEFORE DELETE ON party_ledger_entries FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- <<< GENERATED GUARDS
