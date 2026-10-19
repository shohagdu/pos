-- 0101 r2_inventory (DB design §9.8 Release 2; SRS-BRN, STK transfers and counts).
--
-- Transfers move stock between two locations, usually of different branches:
-- draft → dispatched (`transfer_out` at the source average) → received
-- (`transfer_in` at the dispatch cost; a shortfall is written off as transit
-- loss) or cancelled. Counts snapshot the system quantity, take the counted
-- quantity and post the differences as `count_adjustment` movements.

CREATE TABLE stock_transfers (
    tenant_id         BINARY(16)    NOT NULL,
    id                BINARY(16)    NOT NULL,
    transfer_no       VARCHAR(40),
    from_branch_id    BINARY(16)    NOT NULL,
    from_location_id  BINARY(16)    NOT NULL,
    to_branch_id      BINARY(16)    NOT NULL,
    to_location_id    BINARY(16)    NOT NULL,
    business_date     DATE          NOT NULL,
    status            VARCHAR(20)   NOT NULL DEFAULT 'draft',
    note              TEXT,
    total_value       DECIMAL(18,2) NOT NULL DEFAULT 0,
    loss_value        DECIMAL(18,2) NOT NULL DEFAULT 0,
    dispatched_at     DATETIME(6),
    dispatched_by     BINARY(16),
    received_at       DATETIME(6),
    received_by       BINARY(16),
    cancelled_at      DATETIME(6),
    cancelled_by      BINARY(16),
    version           INT           NOT NULL DEFAULT 1,
    created_at        DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at        DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by        BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_transfers_no (tenant_id, from_branch_id, transfer_no),
    KEY idx_transfers_status (tenant_id, status, created_at),
    CONSTRAINT fk_transfers_from_branch FOREIGN KEY (tenant_id, from_branch_id) REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_transfers_to_branch   FOREIGN KEY (tenant_id, to_branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_transfers_from FOREIGN KEY (tenant_id, from_location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT fk_transfers_to   FOREIGN KEY (tenant_id, to_location_id)   REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT ck_transfers_locs   CHECK (from_location_id <> to_location_id),
    CONSTRAINT ck_transfers_status CHECK (status IN ('draft','dispatched','received','cancelled'))
);

CREATE TABLE stock_transfer_items (
    tenant_id          BINARY(16)    NOT NULL,
    id                 BINARY(16)    NOT NULL,
    transfer_id        BINARY(16)    NOT NULL,
    variant_id         BINARY(16)    NOT NULL,
    quantity_sent      DECIMAL(18,3) NOT NULL,
    quantity_received  DECIMAL(18,3),
    unit_cost          DECIMAL(18,4),                -- source average at dispatch
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_transfer_items (tenant_id, transfer_id, variant_id),
    CONSTRAINT fk_transfer_items_transfer FOREIGN KEY (tenant_id, transfer_id) REFERENCES stock_transfers (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_transfer_items_variant  FOREIGN KEY (tenant_id, variant_id)  REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_transfer_items_qty      CHECK (quantity_sent > 0 AND (quantity_received IS NULL OR quantity_received >= 0))
);

CREATE TABLE stock_counts (
    tenant_id       BINARY(16)    NOT NULL,
    id              BINARY(16)    NOT NULL,
    count_no        VARCHAR(40),
    branch_id       BINARY(16)    NOT NULL,
    location_id     BINARY(16)    NOT NULL,
    category_id     BINARY(16),                  -- NULL = every stocked product
    status          VARCHAR(20)   NOT NULL DEFAULT 'draft',
    note            VARCHAR(500),
    business_date   DATE          NOT NULL,
    variance_value  DECIMAL(18,2) NOT NULL DEFAULT 0,
    snapshot_at     DATETIME(6),
    posted_at       DATETIME(6),
    posted_by       BINARY(16),
    version         INT           NOT NULL DEFAULT 1,
    created_at      DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_by      BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_counts_no (tenant_id, branch_id, count_no),
    KEY idx_counts_status (tenant_id, status, created_at),
    CONSTRAINT fk_counts_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_counts_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT fk_counts_category FOREIGN KEY (tenant_id, category_id) REFERENCES categories (tenant_id, id),
    CONSTRAINT ck_counts_status   CHECK (status IN ('draft','in_progress','review','posted','cancelled'))
);

CREATE TABLE stock_count_items (
    tenant_id     BINARY(16)    NOT NULL,
    id            BINARY(16)    NOT NULL,
    count_id      BINARY(16)    NOT NULL,
    variant_id    BINARY(16)    NOT NULL,
    system_qty    DECIMAL(18,3) NOT NULL,
    counted_qty   DECIMAL(18,3),
    variance_qty  DECIMAL(18,3) AS (counted_qty - system_qty) STORED,
    unit_cost     DECIMAL(18,4),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_count_items (tenant_id, count_id, variant_id),
    CONSTRAINT fk_count_items_count   FOREIGN KEY (tenant_id, count_id)   REFERENCES stock_counts (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_count_items_variant FOREIGN KEY (tenant_id, variant_id) REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_count_items_qty     CHECK (counted_qty IS NULL OR counted_qty >= 0)
);

-- Numbering for counts and other income on branches that already exist.
SET @app_platform = 1;

INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding)
SELECT b.tenant_id, UUID_TO_BIN(UUID(), 1), b.id, d.doc_type, d.prefix, TRUE, 5
FROM branches b
CROSS JOIN (SELECT 'count' AS doc_type, 'CNT' AS prefix UNION ALL SELECT 'income', 'INC') d
WHERE NOT EXISTS (SELECT 1 FROM document_sequences s
                  WHERE s.tenant_id = b.tenant_id AND s.branch_id = b.id AND s.doc_type = d.doc_type);

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- stock_transfers
CREATE TRIGGER trg_stock_transfers_tg_ins BEFORE INSERT ON stock_transfers FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_transfers_tg_upd BEFORE UPDATE ON stock_transfers FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_transfers_tg_del BEFORE DELETE ON stock_transfers FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_transfer_items
CREATE TRIGGER trg_stock_transfer_items_tg_ins BEFORE INSERT ON stock_transfer_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_transfer_items_tg_upd BEFORE UPDATE ON stock_transfer_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_transfer_items_tg_del BEFORE DELETE ON stock_transfer_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_counts
CREATE TRIGGER trg_stock_counts_tg_ins BEFORE INSERT ON stock_counts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_counts_tg_upd BEFORE UPDATE ON stock_counts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_counts_tg_del BEFORE DELETE ON stock_counts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_count_items
CREATE TRIGGER trg_stock_count_items_tg_ins BEFORE INSERT ON stock_count_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_count_items_tg_upd BEFORE UPDATE ON stock_count_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_count_items_tg_del BEFORE DELETE ON stock_count_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
