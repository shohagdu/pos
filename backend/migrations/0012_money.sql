-- 0012 money: payment accounts, payment methods, payments with allocations,
-- the cash/bank book (account_transactions, append-only) and account
-- transfers (DB design §9.10). Balances change only through pos_money.
-- Existing businesses get the provisioning defaults at the end of the file.

CREATE TABLE payment_accounts (
    tenant_id          BINARY(16)    NOT NULL,
    id                 BINARY(16)    NOT NULL,
    branch_id          BINARY(16),
    account_type       VARCHAR(20)   NOT NULL,
    name               VARCHAR(100)  NOT NULL,
    account_number     VARCHAR(50),
    bank_name          VARCHAR(100),
    opening_balance    DECIMAL(18,2) NOT NULL DEFAULT 0,
    current_balance    DECIMAL(18,2) NOT NULL DEFAULT 0,
    ledger_account_id  BINARY(16),
    is_default         BOOLEAN       NOT NULL DEFAULT FALSE,
    status             VARCHAR(20)   NOT NULL DEFAULT 'active',
    version            INT           NOT NULL DEFAULT 1,
    created_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_payment_accounts_name (tenant_id, name),
    CONSTRAINT fk_payment_accounts_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_payment_accounts_branch FOREIGN KEY (tenant_id, branch_id) REFERENCES branches (tenant_id, id),
    CONSTRAINT ck_payment_accounts_type   CHECK (account_type IN ('cash','bank','mobile_wallet','card')),
    CONSTRAINT ck_payment_accounts_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE payment_methods (
    tenant_id           BINARY(16)  NOT NULL,
    id                  BINARY(16)  NOT NULL,
    name                VARCHAR(50) NOT NULL,
    method_type         VARCHAR(20) NOT NULL,
    default_account_id  BINARY(16),
    requires_reference  BOOLEAN     NOT NULL DEFAULT FALSE,
    sort_order          INT         NOT NULL DEFAULT 0,
    is_active           BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at          DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at          DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_payment_methods_name (tenant_id, name),
    CONSTRAINT fk_payment_methods_tenant  FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_payment_methods_account FOREIGN KEY (tenant_id, default_account_id) REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT ck_payment_methods_type    CHECK (method_type IN ('cash','card','mobile_wallet','bank_transfer',
                                                                 'cheque','customer_credit','other'))
);

CREATE TABLE payments (
    tenant_id           BINARY(16)    NOT NULL,
    id                  BINARY(16)    NOT NULL,
    branch_id           BINARY(16)    NOT NULL,
    payment_no          VARCHAR(40)   NOT NULL,
    direction           VARCHAR(3)    NOT NULL,
    payment_date        DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    business_date       DATE          NOT NULL,
    party_type          VARCHAR(10),
    party_id            BINARY(16),
    account_id          BINARY(16)    NOT NULL,
    payment_method_id   BINARY(16)    NOT NULL,
    amount              DECIMAL(18,2) NOT NULL,
    unallocated_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    reference           VARCHAR(100),
    note                VARCHAR(500),
    source              VARCHAR(20)   NOT NULL,
    status              VARCHAR(10)   NOT NULL DEFAULT 'posted',
    voided_at           DATETIME(6),
    voided_by           BINARY(16),
    void_reason         VARCHAR(500),
    register_shift_id   BINARY(16),
    created_by          BINARY(16)    NOT NULL,
    created_at          DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_payments_no (tenant_id, branch_id, payment_no),
    KEY idx_payments_party (tenant_id, party_type, party_id, payment_date),
    KEY idx_payments_date (tenant_id, branch_id, business_date),
    KEY idx_payments_account (tenant_id, account_id, business_date),
    CONSTRAINT fk_payments_branch  FOREIGN KEY (tenant_id, branch_id)         REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_payments_account FOREIGN KEY (tenant_id, account_id)        REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT fk_payments_method  FOREIGN KEY (tenant_id, payment_method_id) REFERENCES payment_methods (tenant_id, id),
    CONSTRAINT ck_payments_direction CHECK (direction IN ('in','out')),
    CONSTRAINT ck_payments_party     CHECK ((party_type IS NULL) = (party_id IS NULL)
                                            AND (party_type IS NULL OR party_type IN ('customer','supplier'))),
    CONSTRAINT ck_payments_amount    CHECK (amount > 0 AND unallocated_amount >= 0 AND unallocated_amount <= amount),
    CONSTRAINT ck_payments_source    CHECK (source IN ('pos','collection','supplier_payment','refund','expense','income','manual')),
    CONSTRAINT ck_payments_status    CHECK (status IN ('posted','voided'))
);

CREATE TABLE payment_allocations (
    tenant_id      BINARY(16)    NOT NULL,
    id             BINARY(16)    NOT NULL,
    payment_id     BINARY(16)    NOT NULL,
    document_type  VARCHAR(20)   NOT NULL,
    document_id    BINARY(16)    NOT NULL,
    amount         DECIMAL(18,2) NOT NULL,
    created_at     DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    reversed_at    DATETIME(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_allocations_document (tenant_id, document_type, document_id),
    KEY idx_allocations_payment (tenant_id, payment_id),
    CONSTRAINT fk_allocations_payment FOREIGN KEY (tenant_id, payment_id) REFERENCES payments (tenant_id, id),
    CONSTRAINT ck_allocations_doc     CHECK (document_type IN ('sale','purchase','sale_return','purchase_return',
                                                               'expense','other_income','opening_balance')),
    CONSTRAINT ck_allocations_amount  CHECK (amount > 0)
);

CREATE TABLE account_transactions (
    tenant_id       BINARY(16)    NOT NULL,
    id              BINARY(16)    NOT NULL,
    account_id      BINARY(16)    NOT NULL,
    branch_id       BINARY(16),
    txn_type        VARCHAR(20)   NOT NULL,
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
    KEY idx_account_txn_book (tenant_id, account_id, business_date, created_at),
    KEY idx_account_txn_ref (tenant_id, reference_type, reference_id),
    CONSTRAINT fk_account_txn_account FOREIGN KEY (tenant_id, account_id) REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT ck_account_txn_type    CHECK (txn_type IN ('opening','payment_in','payment_out','payment_void',
                                                          'transfer_in','transfer_out','expense','income','adjustment')),
    CONSTRAINT ck_account_txn_amount  CHECK (amount <> 0)
);

CREATE TABLE account_transfers (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    branch_id        BINARY(16)    NOT NULL,
    transfer_no      VARCHAR(40)   NOT NULL,
    from_account_id  BINARY(16)    NOT NULL,
    to_account_id    BINARY(16)    NOT NULL,
    amount           DECIMAL(18,2) NOT NULL,
    fee_amount       DECIMAL(18,2) NOT NULL DEFAULT 0,
    business_date    DATE          NOT NULL,
    reference        VARCHAR(100),
    note             VARCHAR(500),
    status           VARCHAR(10)   NOT NULL DEFAULT 'posted',
    created_by       BINARY(16)    NOT NULL,
    created_at       DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_account_transfers_no (tenant_id, transfer_no),
    KEY idx_account_transfers_date (tenant_id, business_date),
    CONSTRAINT fk_acc_transfers_branch FOREIGN KEY (tenant_id, branch_id)       REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_acc_transfers_from   FOREIGN KEY (tenant_id, from_account_id) REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT fk_acc_transfers_to     FOREIGN KEY (tenant_id, to_account_id)   REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT ck_acc_transfers_accts  CHECK (from_account_id <> to_account_id),
    CONSTRAINT ck_acc_transfers_amount CHECK (amount > 0 AND fee_amount >= 0),
    CONSTRAINT ck_acc_transfers_status CHECK (status IN ('posted','voided'))
);

-- Defaults for businesses created before this migration (new ones get them
-- from provisioning). The guard triggers need the platform flag for this.
SET @app_platform = 1;

INSERT INTO payment_accounts (tenant_id, id, account_type, name, is_default)
SELECT t.id, UUID_TO_BIN(UUID(), 1), a.account_type, a.name, a.is_default
FROM tenants t
CROSS JOIN (SELECT 'cash' AS account_type, 'Cash' AS name, TRUE AS is_default
            UNION ALL SELECT 'bank', 'Bank Account', FALSE
            UNION ALL SELECT 'mobile_wallet', 'Mobile Wallet', FALSE) a
WHERE t.slug <> 'platform';

INSERT INTO payment_methods (tenant_id, id, name, method_type, default_account_id, requires_reference, sort_order)
SELECT t.id, UUID_TO_BIN(UUID(), 1), m.name, m.method_type,
       (SELECT pa.id FROM payment_accounts pa WHERE pa.tenant_id = t.id AND pa.name = m.account_name),
       m.requires_reference, m.sort_order
FROM tenants t
CROSS JOIN (SELECT 'Cash' AS name, 'cash' AS method_type, 'Cash' AS account_name, FALSE AS requires_reference, 1 AS sort_order
            UNION ALL SELECT 'Card', 'card', 'Bank Account', FALSE, 2
            UNION ALL SELECT 'bKash', 'mobile_wallet', 'Mobile Wallet', TRUE, 3
            UNION ALL SELECT 'Nagad', 'mobile_wallet', 'Mobile Wallet', TRUE, 4
            UNION ALL SELECT 'Rocket', 'mobile_wallet', 'Mobile Wallet', TRUE, 5
            UNION ALL SELECT 'Bank Transfer', 'bank_transfer', 'Bank Account', TRUE, 6) m
WHERE t.slug <> 'platform';

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- payment_accounts
CREATE TRIGGER trg_payment_accounts_tg_ins BEFORE INSERT ON payment_accounts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payment_accounts_tg_upd BEFORE UPDATE ON payment_accounts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payment_accounts_tg_del BEFORE DELETE ON payment_accounts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- payment_methods
CREATE TRIGGER trg_payment_methods_tg_ins BEFORE INSERT ON payment_methods FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payment_methods_tg_upd BEFORE UPDATE ON payment_methods FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payment_methods_tg_del BEFORE DELETE ON payment_methods FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- payments
CREATE TRIGGER trg_payments_tg_ins BEFORE INSERT ON payments FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payments_tg_upd BEFORE UPDATE ON payments FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payments_tg_del BEFORE DELETE ON payments FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- payment_allocations
CREATE TRIGGER trg_payment_allocations_tg_ins BEFORE INSERT ON payment_allocations FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payment_allocations_tg_upd BEFORE UPDATE ON payment_allocations FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_payment_allocations_tg_del BEFORE DELETE ON payment_allocations FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- account_transactions
CREATE TRIGGER trg_account_transactions_tg_ins BEFORE INSERT ON account_transactions FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_account_transactions_ao_upd BEFORE UPDATE ON account_transactions FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_account_transactions_ao_del BEFORE DELETE ON account_transactions FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- account_transfers
CREATE TRIGGER trg_account_transfers_tg_ins BEFORE INSERT ON account_transfers FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_account_transfers_tg_upd BEFORE UPDATE ON account_transfers FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_account_transfers_tg_del BEFORE DELETE ON account_transfers FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
