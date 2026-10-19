-- 0010 inventory: stock balances (cache), the append-only stock ledger and
-- stock adjustments (DB design §9.8). Balances change only through
-- pos_inventory::post_movements inside the posting transaction (STK-R01).
-- stock_movements is partitioned by month and has no foreign keys (MySQL
-- restriction); the worker keeps three future partitions.

CREATE TABLE stock_balances (
    tenant_id         BINARY(16)    NOT NULL,
    location_id       BINARY(16)    NOT NULL,
    variant_id        BINARY(16)    NOT NULL,
    quantity          DECIMAL(18,3) NOT NULL DEFAULT 0,
    avg_cost          DECIMAL(18,4) NOT NULL DEFAULT 0,
    last_movement_at  DATETIME(6),
    updated_at        DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, location_id, variant_id),
    KEY idx_stock_balances_variant (tenant_id, variant_id),
    CONSTRAINT fk_stock_balances_tenant   FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_stock_balances_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT fk_stock_balances_variant  FOREIGN KEY (tenant_id, variant_id)  REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_stock_balances_cost     CHECK (avg_cost >= 0)
);

CREATE TABLE stock_movements (
    tenant_id          BINARY(16)    NOT NULL,
    variant_id         BINARY(16)    NOT NULL,
    location_id        BINARY(16)    NOT NULL,
    created_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    id                 BINARY(16)    NOT NULL,
    branch_id          BINARY(16)    NOT NULL,
    movement_type      VARCHAR(20)   NOT NULL,
    quantity           DECIMAL(18,3) NOT NULL,
    unit_cost          DECIMAL(18,4) NOT NULL,
    total_cost         DECIMAL(18,2) NOT NULL,
    balance_qty_after  DECIMAL(18,3) NOT NULL,
    avg_cost_after     DECIMAL(18,4) NOT NULL,
    reference_type     VARCHAR(30)   NOT NULL,
    reference_id       BINARY(16)    NOT NULL,
    reference_line_id  BINARY(16),
    reference_no       VARCHAR(40),
    reason             VARCHAR(255),
    batch_id           BINARY(16),
    business_date      DATE          NOT NULL,
    created_by         BINARY(16),
    PRIMARY KEY (tenant_id, variant_id, location_id, created_at, id),
    KEY idx_movements_reference (tenant_id, reference_type, reference_id),
    KEY idx_movements_date (tenant_id, business_date),
    CONSTRAINT ck_movements_type CHECK (movement_type IN ('opening','purchase_receipt','purchase_return',
                         'sale','sale_cancel','sale_return','adjustment','transfer_out','transfer_in',
                         'count_adjustment','damage_move')),
    CONSTRAINT ck_movements_qty  CHECK (quantity <> 0),
    CONSTRAINT ck_movements_cost CHECK (unit_cost >= 0)
)
PARTITION BY RANGE COLUMNS (created_at) (
    PARTITION p2026_10 VALUES LESS THAN ('2026-11-01'),
    PARTITION p2026_11 VALUES LESS THAN ('2026-12-01'),
    PARTITION p2026_12 VALUES LESS THAN ('2027-01-01'),
    PARTITION p2027_01 VALUES LESS THAN ('2027-02-01'),
    PARTITION pmax     VALUES LESS THAN (MAXVALUE)
);

CREATE TABLE stock_adjustments (
    tenant_id      BINARY(16)    NOT NULL,
    id             BINARY(16)    NOT NULL,
    branch_id      BINARY(16)    NOT NULL,
    location_id    BINARY(16)    NOT NULL,
    adjustment_no  VARCHAR(40),
    business_date  DATE          NOT NULL,
    reason         VARCHAR(20)   NOT NULL,
    note           TEXT,
    status         VARCHAR(20)   NOT NULL DEFAULT 'draft',
    total_value    DECIMAL(18,2) NOT NULL DEFAULT 0,
    posted_at      DATETIME(6),
    posted_by      BINARY(16),
    created_at     DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at     DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by     BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_adjustments_no (tenant_id, branch_id, adjustment_no),
    KEY idx_adjustments_date (tenant_id, business_date),
    CONSTRAINT fk_adjustments_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_adjustments_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT ck_adjustments_reason   CHECK (reason IN ('opening','damaged','lost','found','expired','correction',
                                                         'internal_use','transit_loss','other')),
    CONSTRAINT ck_adjustments_status   CHECK (status IN ('draft','posted','cancelled'))
);

CREATE TABLE stock_adjustment_items (
    tenant_id      BINARY(16)    NOT NULL,
    id             BINARY(16)    NOT NULL,
    adjustment_id  BINARY(16)    NOT NULL,
    variant_id     BINARY(16)    NOT NULL,
    quantity       DECIMAL(18,3) NOT NULL,
    unit_cost      DECIMAL(18,4),
    note           VARCHAR(255),
    PRIMARY KEY (tenant_id, id),
    KEY idx_adj_items_adjustment (tenant_id, adjustment_id),
    CONSTRAINT fk_adj_items_adjustment FOREIGN KEY (tenant_id, adjustment_id) REFERENCES stock_adjustments (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_adj_items_variant    FOREIGN KEY (tenant_id, variant_id)    REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_adj_items_qty        CHECK (quantity <> 0)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- stock_balances
CREATE TRIGGER trg_stock_balances_tg_ins BEFORE INSERT ON stock_balances FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_balances_tg_upd BEFORE UPDATE ON stock_balances FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_balances_tg_del BEFORE DELETE ON stock_balances FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_movements
CREATE TRIGGER trg_stock_movements_tg_ins BEFORE INSERT ON stock_movements FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_movements_ao_upd BEFORE UPDATE ON stock_movements FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_stock_movements_ao_del BEFORE DELETE ON stock_movements FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- stock_adjustments
CREATE TRIGGER trg_stock_adjustments_tg_ins BEFORE INSERT ON stock_adjustments FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_adjustments_tg_upd BEFORE UPDATE ON stock_adjustments FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_adjustments_tg_del BEFORE DELETE ON stock_adjustments FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_adjustment_items
CREATE TRIGGER trg_stock_adjustment_items_tg_ins BEFORE INSERT ON stock_adjustment_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_adjustment_items_tg_upd BEFORE UPDATE ON stock_adjustment_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_adjustment_items_tg_del BEFORE DELETE ON stock_adjustment_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
