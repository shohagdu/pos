-- 0108 bank reconciliation (PAY-R06).
--
-- A reconciliation matches a bank or wallet account's book with one bank
-- statement: the statement date and closing balance are entered, the account
-- transactions on the statement are ticked as cleared, and it can be
-- completed only when the reconciled opening balance plus the ticked items
-- equals the statement balance. The book (`account_transactions`) is
-- append-only, so the ticks live in `bank_reconciliation_items`; a
-- transaction can be cleared once (unique key). Opening-balance rows count as
-- cleared from the start.

CREATE TABLE bank_reconciliations (
    tenant_id          BINARY(16)    NOT NULL,
    id                 BINARY(16)    NOT NULL,
    account_id         BINARY(16)    NOT NULL,
    statement_date     DATE          NOT NULL,
    statement_balance  DECIMAL(18,2) NOT NULL,
    -- Cleared balance carried from earlier reconciliations when this one started.
    opening_balance    DECIMAL(18,2) NOT NULL,
    -- Set on completion.
    cleared_balance    DECIMAL(18,2),
    status             VARCHAR(20)   NOT NULL DEFAULT 'in_progress',
    note               VARCHAR(500),
    created_by         BINARY(16)    NOT NULL,
    created_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    completed_at       DATETIME(6),
    completed_by       BINARY(16),
    version            INT           NOT NULL DEFAULT 1,
    -- One reconciliation in progress per account.
    open_account_id    BINARY(16) GENERATED ALWAYS AS (IF(status = 'in_progress', account_id, NULL)) STORED,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_bank_rec_open (tenant_id, open_account_id),
    KEY idx_bank_rec_account (tenant_id, account_id, statement_date),
    CONSTRAINT fk_bank_rec_account FOREIGN KEY (tenant_id, account_id) REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT ck_bank_rec_status  CHECK (status IN ('in_progress', 'completed', 'cancelled'))
);

CREATE TABLE bank_reconciliation_items (
    tenant_id               BINARY(16)    NOT NULL,
    id                      BINARY(16)    NOT NULL,
    reconciliation_id       BINARY(16)    NOT NULL,
    account_transaction_id  BINARY(16)    NOT NULL,
    amount                  DECIMAL(18,2) NOT NULL,
    created_at              DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_bank_rec_items_txn (tenant_id, account_transaction_id),
    KEY idx_bank_rec_items_rec (tenant_id, reconciliation_id),
    CONSTRAINT fk_bank_rec_items_rec FOREIGN KEY (tenant_id, reconciliation_id) REFERENCES bank_reconciliations (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_bank_rec_items_txn FOREIGN KEY (tenant_id, account_transaction_id) REFERENCES account_transactions (tenant_id, id)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- bank_reconciliations
CREATE TRIGGER trg_bank_reconciliations_tg_ins BEFORE INSERT ON bank_reconciliations FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_bank_reconciliations_tg_upd BEFORE UPDATE ON bank_reconciliations FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_bank_reconciliations_tg_del BEFORE DELETE ON bank_reconciliations FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- bank_reconciliation_items
CREATE TRIGGER trg_bank_reconciliation_items_tg_ins BEFORE INSERT ON bank_reconciliation_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_bank_reconciliation_items_tg_upd BEFORE UPDATE ON bank_reconciliation_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_bank_reconciliation_items_tg_del BEFORE DELETE ON bank_reconciliation_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
