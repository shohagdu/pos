-- 0117 clear all transactions, with revert.
-- The owner can clear every transaction of the business (sales, returns,
-- purchases, payments, expenses, incomes, stock movements, journals, shifts,
-- held carts, reconciliations, fiscal periods) in one step. Master data stays.
-- Nothing is lost: the cleared rows are copied into `data_archive_rows` (one
-- JSON object per row) together with the cached values that were reset
-- (dues, account balances, last purchase prices, document numbers).
-- Restoring an archive first archives whatever is live now, so a revert never
-- loses work done after the clear either (crates/tenancy/src/reset.rs).

CREATE TABLE data_archives (
    tenant_id    BINARY(16)   NOT NULL,
    id           BINARY(16)   NOT NULL,
    -- `clear`: made by "Clear all transactions"; `restore`: the live data set
    -- aside when another archive was restored.
    kind         VARCHAR(10)  NOT NULL,
    -- `held` while it can be restored; `restored` once its rows are live again.
    status       VARCHAR(10)  NOT NULL DEFAULT 'held',
    -- Rows per table, e.g. {"sales": 12, "sale_items": 30}.
    summary      JSON         NOT NULL,
    -- The columns archived per table, so a later schema change can still restore.
    columns_json JSON         NOT NULL,
    created_by   BINARY(16)   NOT NULL,
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    restored_by  BINARY(16),
    restored_at  DATETIME(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_data_archives_created (tenant_id, created_at),
    CONSTRAINT fk_data_archives_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_data_archives_kind   CHECK (kind IN ('clear','restore')),
    CONSTRAINT ck_data_archives_status CHECK (status IN ('held','restored'))
);

CREATE TABLE data_archive_rows (
    tenant_id   BINARY(16)  NOT NULL,
    archive_id  BINARY(16)  NOT NULL,
    table_name  VARCHAR(64) NOT NULL,
    -- Insertion order within the table (parents before children).
    row_no      BIGINT      NOT NULL,
    -- The row as a JSON object; binary columns as hex strings.
    row_data    JSON        NOT NULL,
    PRIMARY KEY (tenant_id, archive_id, table_name, row_no),
    CONSTRAINT fk_data_archive_rows_archive FOREIGN KEY (tenant_id, archive_id) REFERENCES data_archives (tenant_id, id)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- data_archives
CREATE TRIGGER trg_data_archives_tg_ins BEFORE INSERT ON data_archives FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_data_archives_tg_upd BEFORE UPDATE ON data_archives FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_data_archives_tg_del BEFORE DELETE ON data_archives FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- data_archive_rows
CREATE TRIGGER trg_data_archive_rows_tg_ins BEFORE INSERT ON data_archive_rows FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_data_archive_rows_tg_upd BEFORE UPDATE ON data_archive_rows FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_data_archive_rows_tg_del BEFORE DELETE ON data_archive_rows FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
