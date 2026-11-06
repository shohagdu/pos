-- 0114 batch and expiry tracking (STK-R11; DB design §17 "Batch & expiry").
-- A batch is a variant's lot with a number and an expiry date. Stock of a
-- variant at a location is its batches plus a "no batch" remainder
-- (stock_balances.quantity minus the batch balances), so products without
-- batches work as before. Batch balances change only in
-- pos_inventory::post_movements: receipts put stock into a batch, outflows
-- take the earliest expiry first (FEFO), and reversals put stock back into
-- the batches it came from (stock_batch_movements.origin_line_id).

ALTER TABLE products
    ADD COLUMN track_batches BOOLEAN NOT NULL DEFAULT FALSE AFTER track_inventory;

CREATE TABLE product_batches (
    tenant_id    BINARY(16)   NOT NULL,
    id           BINARY(16)   NOT NULL,
    variant_id   BINARY(16)   NOT NULL,
    batch_no     VARCHAR(40)  NOT NULL,
    expiry_date  DATE,
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_by   BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_product_batches_no (tenant_id, variant_id, batch_no),
    KEY idx_product_batches_expiry (tenant_id, expiry_date),
    CONSTRAINT fk_product_batches_tenant  FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_product_batches_variant FOREIGN KEY (tenant_id, variant_id) REFERENCES product_variants (tenant_id, id)
);

CREATE TABLE stock_batch_balances (
    tenant_id    BINARY(16)    NOT NULL,
    location_id  BINARY(16)    NOT NULL,
    batch_id     BINARY(16)    NOT NULL,
    variant_id   BINARY(16)    NOT NULL,
    quantity     DECIMAL(18,3) NOT NULL DEFAULT 0,
    updated_at   DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, location_id, batch_id),
    KEY idx_batch_balances_variant (tenant_id, location_id, variant_id),
    CONSTRAINT fk_batch_balances_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT fk_batch_balances_batch    FOREIGN KEY (tenant_id, batch_id)    REFERENCES product_batches (tenant_id, id),
    CONSTRAINT fk_batch_balances_variant  FOREIGN KEY (tenant_id, variant_id)  REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_batch_balances_qty      CHECK (quantity >= 0)
);

-- Which batches each stock movement line moved. `origin_line_id` is the
-- document line a reversal undoes (the sale line a return puts back, the
-- purchase line a supplier return takes from, the transfer line received).
CREATE TABLE stock_batch_movements (
    tenant_id          BINARY(16)    NOT NULL,
    id                 BINARY(16)    NOT NULL,
    batch_id           BINARY(16)    NOT NULL,
    variant_id         BINARY(16)    NOT NULL,
    location_id        BINARY(16)    NOT NULL,
    quantity           DECIMAL(18,3) NOT NULL,
    reference_type     VARCHAR(30)   NOT NULL,
    reference_id       BINARY(16)    NOT NULL,
    reference_line_id  BINARY(16),
    origin_line_id     BINARY(16),
    created_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_batch_movements_line (tenant_id, reference_line_id),
    KEY idx_batch_movements_origin (tenant_id, origin_line_id),
    KEY idx_batch_movements_reference (tenant_id, reference_type, reference_id),
    CONSTRAINT fk_batch_movements_batch FOREIGN KEY (tenant_id, batch_id) REFERENCES product_batches (tenant_id, id),
    CONSTRAINT ck_batch_movements_qty   CHECK (quantity <> 0)
);

ALTER TABLE stock_adjustment_items
    ADD COLUMN batch_id BINARY(16) AFTER unit_cost,
    ADD CONSTRAINT fk_adj_items_batch FOREIGN KEY (tenant_id, batch_id) REFERENCES product_batches (tenant_id, id);

-- The batch a purchase line arrives as; a receipt may name another.
ALTER TABLE purchase_items
    ADD COLUMN batch_no    VARCHAR(40),
    ADD COLUMN expiry_date DATE;

ALTER TABLE goods_receipt_items
    ADD COLUMN batch_id BINARY(16),
    ADD CONSTRAINT fk_grn_items_batch FOREIGN KEY (tenant_id, batch_id) REFERENCES product_batches (tenant_id, id);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- product_batches
CREATE TRIGGER trg_product_batches_tg_ins BEFORE INSERT ON product_batches FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_batches_tg_upd BEFORE UPDATE ON product_batches FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_batches_tg_del BEFORE DELETE ON product_batches FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_batch_balances
CREATE TRIGGER trg_stock_batch_balances_tg_ins BEFORE INSERT ON stock_batch_balances FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_batch_balances_tg_upd BEFORE UPDATE ON stock_batch_balances FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_batch_balances_tg_del BEFORE DELETE ON stock_batch_balances FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- stock_batch_movements
CREATE TRIGGER trg_stock_batch_movements_tg_ins BEFORE INSERT ON stock_batch_movements FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_stock_batch_movements_ao_upd BEFORE UPDATE ON stock_batch_movements FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_stock_batch_movements_ao_del BEFORE DELETE ON stock_batch_movements FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- <<< GENERATED GUARDS
