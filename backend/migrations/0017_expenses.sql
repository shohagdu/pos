-- 0017 expenses (DB design §9.14, EXP-R01). An expense creates an outgoing
-- payment from the chosen account. Attachments, approval and recurring
-- templates are Release 2.

CREATE TABLE expense_categories (
    tenant_id          BINARY(16)   NOT NULL,
    id                 BINARY(16)   NOT NULL,
    name               VARCHAR(100) NOT NULL,
    ledger_account_id  BINARY(16),
    is_system          BOOLEAN      NOT NULL DEFAULT FALSE,
    status             VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at         DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_expense_categories_name (tenant_id, name),
    CONSTRAINT fk_expense_categories_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_expense_categories_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE expenses (
    tenant_id           BINARY(16)    NOT NULL,
    id                  BINARY(16)    NOT NULL,
    branch_id           BINARY(16)    NOT NULL,
    expense_no          VARCHAR(40)   NOT NULL,
    category_id         BINARY(16)    NOT NULL,
    business_date       DATE          NOT NULL,
    amount              DECIMAL(18,2) NOT NULL,
    tax_amount          DECIMAL(18,2) NOT NULL DEFAULT 0,
    account_id          BINARY(16)    NOT NULL,
    payment_id          BINARY(16),
    payee               VARCHAR(150),
    reference           VARCHAR(100),
    note                VARCHAR(500),
    attachment_file_id  BINARY(16),
    status              VARCHAR(20)   NOT NULL DEFAULT 'posted',
    approved_by         BINARY(16),
    recurring_rule_id   BINARY(16),
    idempotency_key     BINARY(16),
    voided_at           DATETIME(6),
    voided_by           BINARY(16),
    void_reason         VARCHAR(500),
    created_by          BINARY(16)    NOT NULL,
    created_at          DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_expenses_no (tenant_id, branch_id, expense_no),
    UNIQUE KEY uq_expenses_idem (tenant_id, idempotency_key),
    KEY idx_expenses_date (tenant_id, branch_id, business_date),
    KEY idx_expenses_category (tenant_id, category_id, business_date),
    CONSTRAINT fk_expenses_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_expenses_category FOREIGN KEY (tenant_id, category_id) REFERENCES expense_categories (tenant_id, id),
    CONSTRAINT fk_expenses_account  FOREIGN KEY (tenant_id, account_id)  REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT fk_expenses_payment  FOREIGN KEY (tenant_id, payment_id)  REFERENCES payments (tenant_id, id),
    CONSTRAINT ck_expenses_amount   CHECK (amount > 0 AND tax_amount >= 0 AND tax_amount <= amount),
    CONSTRAINT ck_expenses_status   CHECK (status IN ('pending_approval','posted','voided','rejected'))
);

-- Default categories for businesses created before this migration.
SET @app_platform = 1;

INSERT INTO expense_categories (tenant_id, id, name, is_system)
SELECT t.id, UUID_TO_BIN(UUID(), 1), c.name, TRUE
FROM tenants t
CROSS JOIN (SELECT 'Rent' AS name UNION ALL SELECT 'Utilities' UNION ALL SELECT 'Salaries'
            UNION ALL SELECT 'Transport' UNION ALL SELECT 'Marketing' UNION ALL SELECT 'Maintenance'
            UNION ALL SELECT 'Bank & MFS Charges' UNION ALL SELECT 'Miscellaneous') c
WHERE t.slug <> 'platform';

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- expense_categories
CREATE TRIGGER trg_expense_categories_tg_ins BEFORE INSERT ON expense_categories FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_expense_categories_tg_upd BEFORE UPDATE ON expense_categories FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_expense_categories_tg_del BEFORE DELETE ON expense_categories FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- expenses
CREATE TRIGGER trg_expenses_tg_ins BEFORE INSERT ON expenses FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_expenses_tg_upd BEFORE UPDATE ON expenses FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_expenses_tg_del BEFORE DELETE ON expenses FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
