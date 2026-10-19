-- 0105 accounting: chart of accounts, fiscal periods and double-entry journals
-- (DB design §9.16, Requirement Analysis §7.13, §8.7). Every business event is
-- journalled in its own transaction by pos_accounting::post_journal, which
-- checks that debits equal credits before writing (ACC-R03).

CREATE TABLE ledger_accounts (
    tenant_id     BINARY(16)   NOT NULL,
    id            BINARY(16)   NOT NULL,
    code          VARCHAR(20)  NOT NULL,
    name          VARCHAR(150) NOT NULL,
    account_type  VARCHAR(10)  NOT NULL,
    normal_side   VARCHAR(6)   NOT NULL,
    parent_id     BINARY(16),
    system_key    VARCHAR(40),
    is_postable   BOOLEAN      NOT NULL DEFAULT TRUE,
    status        VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_ledger_code (tenant_id, code),
    UNIQUE KEY uq_ledger_system_key (tenant_id, system_key),
    CONSTRAINT fk_ledger_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_ledger_parent FOREIGN KEY (tenant_id, parent_id) REFERENCES ledger_accounts (tenant_id, id),
    CONSTRAINT ck_ledger_type   CHECK (account_type IN ('asset','liability','equity','income','expense')),
    CONSTRAINT ck_ledger_side   CHECK (normal_side IN ('debit','credit')),
    CONSTRAINT ck_ledger_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE fiscal_periods (
    tenant_id   BINARY(16)  NOT NULL,
    id          BINARY(16)  NOT NULL,
    start_date  DATE        NOT NULL,
    end_date    DATE        NOT NULL,
    status      VARCHAR(10) NOT NULL DEFAULT 'open',
    locked_at   DATETIME(6),
    locked_by   BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_fiscal_periods_start (tenant_id, start_date),
    CONSTRAINT fk_fiscal_periods_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_fiscal_periods_dates  CHECK (end_date >= start_date),
    CONSTRAINT ck_fiscal_periods_status CHECK (status IN ('open','locked'))
);

CREATE TABLE journal_entries (
    tenant_id       BINARY(16)    NOT NULL,
    id              BINARY(16)    NOT NULL,
    branch_id       BINARY(16),
    entry_no        VARCHAR(40)   NOT NULL,
    entry_date      DATE          NOT NULL,
    source_type     VARCHAR(30)   NOT NULL,
    source_id       BINARY(16),
    source_no       VARCHAR(40),
    memo            VARCHAR(500),
    total_debit     DECIMAL(18,2) NOT NULL,
    total_credit    DECIMAL(18,2) NOT NULL,
    status          VARCHAR(10)   NOT NULL DEFAULT 'posted',
    reversal_of_id  BINARY(16),
    created_by      BINARY(16),
    created_at      DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_journal_entries_no (tenant_id, entry_no),
    KEY idx_journal_source (tenant_id, source_type, source_id),
    KEY idx_journal_date (tenant_id, entry_date),
    CONSTRAINT fk_journal_tenant   FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_journal_reversal FOREIGN KEY (tenant_id, reversal_of_id) REFERENCES journal_entries (tenant_id, id),
    CONSTRAINT ck_journal_balanced CHECK (total_debit = total_credit AND total_debit > 0),
    CONSTRAINT ck_journal_status   CHECK (status IN ('posted','reversed'))
);

CREATE TABLE journal_lines (
    tenant_id   BINARY(16)    NOT NULL,
    id          BINARY(16)    NOT NULL,
    entry_id    BINARY(16)    NOT NULL,
    entry_date  DATE          NOT NULL,
    account_id  BINARY(16)    NOT NULL,
    debit       DECIMAL(18,2) NOT NULL DEFAULT 0,
    credit      DECIMAL(18,2) NOT NULL DEFAULT 0,
    party_type  VARCHAR(10),
    party_id    BINARY(16),
    branch_id   BINARY(16),
    memo        VARCHAR(255),
    PRIMARY KEY (tenant_id, id),
    KEY idx_journal_lines_account (tenant_id, account_id, entry_date),
    KEY idx_journal_lines_entry (tenant_id, entry_id),
    CONSTRAINT fk_journal_lines_entry   FOREIGN KEY (tenant_id, entry_id)   REFERENCES journal_entries (tenant_id, id),
    CONSTRAINT fk_journal_lines_account FOREIGN KEY (tenant_id, account_id) REFERENCES ledger_accounts (tenant_id, id),
    CONSTRAINT ck_journal_lines_sides   CHECK (debit >= 0 AND credit >= 0 AND ((debit = 0) <> (credit = 0)))
);

ALTER TABLE payment_accounts
    ADD CONSTRAINT fk_payment_accounts_ledger FOREIGN KEY (tenant_id, ledger_account_id) REFERENCES ledger_accounts (tenant_id, id);
ALTER TABLE expense_categories
    ADD CONSTRAINT fk_expense_categories_ledger FOREIGN KEY (tenant_id, ledger_account_id) REFERENCES ledger_accounts (tenant_id, id);

-- Postings dated in a locked period are refused (ACC-R05, 423 PERIOD_LOCKED).
CREATE TRIGGER trg_journal_entries_period BEFORE INSERT ON journal_entries FOR EACH ROW
BEGIN
  IF EXISTS (SELECT 1 FROM fiscal_periods
             WHERE tenant_id = NEW.tenant_id AND status = 'locked'
               AND NEW.entry_date BETWEEN start_date AND end_date) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'PERIOD_LOCKED';
  END IF;
END;

-- Chart of accounts, links and journal numbers for businesses created before
-- this migration. New businesses get the same from provisioning
-- (crates/tenancy/src/provisioning.rs, CHART_OF_ACCOUNTS).
SET @app_platform = 1;

CREATE TEMPORARY TABLE tmp_coa (
    code         VARCHAR(20),
    name         VARCHAR(150),
    account_type VARCHAR(10),
    normal_side  VARCHAR(6),
    parent_code  VARCHAR(20),
    system_key   VARCHAR(40),
    is_postable  BOOLEAN
);
INSERT INTO tmp_coa VALUES
    ('1000', 'Assets',                  'asset',     'debit',  NULL,   NULL,                   FALSE),
    ('1010', 'Cash in Hand',            'asset',     'debit',  '1000', 'cash',                 TRUE),
    ('1020', 'Bank Accounts',           'asset',     'debit',  '1000', 'bank',                 TRUE),
    ('1030', 'Mobile Wallets',          'asset',     'debit',  '1000', 'mobile_wallet',        TRUE),
    ('1100', 'Accounts Receivable',     'asset',     'debit',  '1000', 'ar',                   TRUE),
    ('1200', 'Inventory',               'asset',     'debit',  '1000', 'inventory',            TRUE),
    ('1300', 'VAT Receivable (Input)',  'asset',     'debit',  '1000', 'vat_receivable',       TRUE),
    ('2000', 'Liabilities',             'liability', 'credit', NULL,   NULL,                   FALSE),
    ('2100', 'Accounts Payable',        'liability', 'credit', '2000', 'ap',                   TRUE),
    ('2200', 'VAT Payable (Output)',    'liability', 'credit', '2000', 'vat_payable',          TRUE),
    ('2300', 'Customer Advances',       'liability', 'credit', '2000', 'customer_advances',    TRUE),
    ('3000', 'Equity',                  'equity',    'credit', NULL,   NULL,                   FALSE),
    ('3100', 'Owner''s Capital',        'equity',    'credit', '3000', 'owner_capital',        TRUE),
    ('3200', 'Opening Balance Equity',  'equity',    'credit', '3000', 'opening_equity',       TRUE),
    ('3300', 'Retained Earnings',       'equity',    'credit', '3000', 'retained_earnings',    TRUE),
    ('4000', 'Income',                  'income',    'credit', NULL,   NULL,                   FALSE),
    ('4100', 'Sales',                   'income',    'credit', '4000', 'sales',                TRUE),
    ('4110', 'Sales Returns',           'income',    'debit',  '4000', 'sales_returns',        TRUE),
    ('4120', 'Sales Discounts',         'income',    'debit',  '4000', 'sales_discount',       TRUE),
    ('4200', 'Other Income',            'income',    'credit', '4000', 'other_income',         TRUE),
    ('4300', 'Rounding Gain/Loss',      'income',    'credit', '4000', 'rounding',             TRUE),
    ('5000', 'Cost of Goods Sold',      'expense',   'debit',  NULL,   'cogs',                 TRUE),
    ('5100', 'Inventory Adjustments',   'expense',   'debit',  NULL,   'inventory_adjustment', TRUE),
    ('6000', 'Operating Expenses',      'expense',   'debit',  NULL,   NULL,                   FALSE),
    ('6010', 'Rent',                    'expense',   'debit',  '6000', NULL,                   TRUE),
    ('6020', 'Utilities',               'expense',   'debit',  '6000', NULL,                   TRUE),
    ('6030', 'Salaries',                'expense',   'debit',  '6000', NULL,                   TRUE),
    ('6040', 'Transport',               'expense',   'debit',  '6000', NULL,                   TRUE),
    ('6050', 'Marketing',               'expense',   'debit',  '6000', NULL,                   TRUE),
    ('6060', 'Maintenance',             'expense',   'debit',  '6000', NULL,                   TRUE),
    ('6070', 'Bank & MFS Charges',      'expense',   'debit',  '6000', 'bank_charges',         TRUE),
    ('6080', 'Miscellaneous',           'expense',   'debit',  '6000', 'misc_expense',         TRUE);

INSERT INTO ledger_accounts (tenant_id, id, code, name, account_type, normal_side, system_key, is_postable)
SELECT t.id, UUID_TO_BIN(UUID(), 1), c.code, c.name, c.account_type, c.normal_side, c.system_key, c.is_postable
FROM tenants t CROSS JOIN tmp_coa c WHERE t.slug <> 'platform';

UPDATE ledger_accounts child
JOIN tmp_coa c ON c.code = child.code AND c.parent_code IS NOT NULL
JOIN ledger_accounts parent ON parent.tenant_id = child.tenant_id AND parent.code = c.parent_code
SET child.parent_id = parent.id;

DROP TEMPORARY TABLE tmp_coa;

UPDATE payment_accounts pa
JOIN ledger_accounts la ON la.tenant_id = pa.tenant_id
 AND la.system_key = CASE pa.account_type WHEN 'cash' THEN 'cash' WHEN 'mobile_wallet' THEN 'mobile_wallet' ELSE 'bank' END
SET pa.ledger_account_id = la.id;

UPDATE expense_categories ec
JOIN ledger_accounts la ON la.tenant_id = ec.tenant_id AND la.name = ec.name AND la.code LIKE '60%'
SET ec.ledger_account_id = la.id;

INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding)
SELECT t.id, UUID_TO_BIN(UUID(), 1), NULL, 'journal', 'JV', TRUE, 5
FROM tenants t
WHERE t.slug <> 'platform' AND NOT EXISTS (SELECT 1 FROM document_sequences s
                  WHERE s.tenant_id = t.id AND s.branch_id IS NULL AND s.doc_type = 'journal');

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- ledger_accounts
CREATE TRIGGER trg_ledger_accounts_tg_ins BEFORE INSERT ON ledger_accounts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_ledger_accounts_tg_upd BEFORE UPDATE ON ledger_accounts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_ledger_accounts_tg_del BEFORE DELETE ON ledger_accounts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- fiscal_periods
CREATE TRIGGER trg_fiscal_periods_tg_ins BEFORE INSERT ON fiscal_periods FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_fiscal_periods_tg_upd BEFORE UPDATE ON fiscal_periods FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_fiscal_periods_tg_del BEFORE DELETE ON fiscal_periods FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- journal_entries
CREATE TRIGGER trg_journal_entries_tg_ins BEFORE INSERT ON journal_entries FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_journal_entries_tg_upd BEFORE UPDATE ON journal_entries FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_journal_entries_tg_del BEFORE DELETE ON journal_entries FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- journal_lines
CREATE TRIGGER trg_journal_lines_tg_ins BEFORE INSERT ON journal_lines FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_journal_lines_ao_upd BEFORE UPDATE ON journal_lines FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_journal_lines_ao_del BEFORE DELETE ON journal_lines FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- <<< GENERATED GUARDS
