-- 0106 r2_money (DB design §9.14 Release 2): other income — money received
-- that is not a sale (interest, rent received, commission, scrap). Each
-- record creates an incoming payment into the chosen account and a journal
-- Dr cash/bank / Cr the category's income account. Voiding reverses both.

CREATE TABLE income_categories (
    tenant_id          BINARY(16)   NOT NULL,
    id                 BINARY(16)   NOT NULL,
    name               VARCHAR(100) NOT NULL,
    ledger_account_id  BINARY(16),
    is_system          BOOLEAN      NOT NULL DEFAULT FALSE,
    status             VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at         DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_income_categories_name (tenant_id, name),
    CONSTRAINT fk_income_categories_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_income_categories_ledger FOREIGN KEY (tenant_id, ledger_account_id) REFERENCES ledger_accounts (tenant_id, id),
    CONSTRAINT ck_income_categories_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE other_incomes (
    tenant_id      BINARY(16)    NOT NULL,
    id             BINARY(16)    NOT NULL,
    branch_id      BINARY(16)    NOT NULL,
    income_no      VARCHAR(40)   NOT NULL,
    category_id    BINARY(16)    NOT NULL,
    business_date  DATE          NOT NULL,
    amount         DECIMAL(18,2) NOT NULL,
    account_id     BINARY(16)    NOT NULL,
    payment_id     BINARY(16),
    source         VARCHAR(150),
    reference      VARCHAR(100),
    note           VARCHAR(500),
    status         VARCHAR(10)   NOT NULL DEFAULT 'posted',
    voided_at      DATETIME(6),
    voided_by      BINARY(16),
    void_reason    VARCHAR(500),
    created_by     BINARY(16)    NOT NULL,
    created_at     DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_other_incomes_no (tenant_id, branch_id, income_no),
    KEY idx_other_incomes_date (tenant_id, business_date),
    CONSTRAINT fk_other_incomes_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_other_incomes_category FOREIGN KEY (tenant_id, category_id) REFERENCES income_categories (tenant_id, id),
    CONSTRAINT fk_other_incomes_account  FOREIGN KEY (tenant_id, account_id)  REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT ck_other_incomes_amount   CHECK (amount > 0),
    CONSTRAINT ck_other_incomes_status   CHECK (status IN ('posted','voided'))
);

-- Default categories for businesses created before this migration (new ones
-- get them from provisioning, crates/tenancy/src/provisioning.rs).
SET @app_platform = 1;

INSERT INTO income_categories (tenant_id, id, name, ledger_account_id, is_system)
SELECT t.id, UUID_TO_BIN(UUID(), 1), c.name,
       (SELECT la.id FROM ledger_accounts la WHERE la.tenant_id = t.id AND la.system_key = 'other_income'), TRUE
FROM tenants t
CROSS JOIN (SELECT 'Interest' AS name UNION ALL SELECT 'Rent Received' UNION ALL SELECT 'Commission'
            UNION ALL SELECT 'Scrap Sales' UNION ALL SELECT 'Miscellaneous Income') c
WHERE t.slug <> 'platform' AND NOT EXISTS (SELECT 1 FROM income_categories ic WHERE ic.tenant_id = t.id AND ic.name = c.name);

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- income_categories
CREATE TRIGGER trg_income_categories_tg_ins BEFORE INSERT ON income_categories FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_income_categories_tg_upd BEFORE UPDATE ON income_categories FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_income_categories_tg_del BEFORE DELETE ON income_categories FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- other_incomes
CREATE TRIGGER trg_other_incomes_tg_ins BEFORE INSERT ON other_incomes FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_other_incomes_tg_upd BEFORE UPDATE ON other_incomes FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_other_incomes_tg_del BEFORE DELETE ON other_incomes FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
