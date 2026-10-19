-- 0016 purchasing: purchase bills with goods receipts (DB design §9.13).
-- MVP posting receives every line at once (DB design §10.4); purchase orders
-- and partial receiving are Release 2.

CREATE TABLE purchases (
    tenant_id             BINARY(16)    NOT NULL,
    id                    BINARY(16)    NOT NULL,
    branch_id             BINARY(16)    NOT NULL,
    location_id           BINARY(16)    NOT NULL,
    purchase_no           VARCHAR(40),
    supplier_id           BINARY(16)    NOT NULL,
    supplier_invoice_no   VARCHAR(60),
    purchase_date         DATE          NOT NULL,
    expected_date         DATE,
    status                VARCHAR(20)   NOT NULL DEFAULT 'draft',
    payment_status        VARCHAR(10)   NOT NULL DEFAULT 'unpaid',
    subtotal              DECIMAL(18,2) NOT NULL DEFAULT 0,
    item_discount_total   DECIMAL(18,2) NOT NULL DEFAULT 0,
    bill_discount_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    tax_total             DECIMAL(18,2) NOT NULL DEFAULT 0,
    shipping_cost         DECIMAL(18,2) NOT NULL DEFAULT 0,
    other_cost            DECIMAL(18,2) NOT NULL DEFAULT 0,
    grand_total           DECIMAL(18,2) NOT NULL DEFAULT 0,
    paid_total            DECIMAL(18,2) NOT NULL DEFAULT 0,
    returned_total        DECIMAL(18,2) NOT NULL DEFAULT 0,
    due_total             DECIMAL(18,2) NOT NULL DEFAULT 0,
    note                  TEXT,
    attachment_file_id    BINARY(16),
    approved_by           BINARY(16),
    approved_at           DATETIME(6),
    posted_at             DATETIME(6),
    posted_by             BINARY(16),
    cancelled_at          DATETIME(6),
    cancelled_by          BINARY(16),
    cancel_reason         VARCHAR(500),
    idempotency_key       BINARY(16),
    version               INT           NOT NULL DEFAULT 1,
    created_by            BINARY(16)    NOT NULL,
    created_at            DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at            DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    live_supplier_invoice VARCHAR(60)   AS (IF(status <> 'cancelled', supplier_invoice_no, NULL)) VIRTUAL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_purchases_no (tenant_id, branch_id, purchase_no),
    UNIQUE KEY uq_purchases_idem (tenant_id, idempotency_key),
    UNIQUE KEY uq_purchases_supplier_invoice (tenant_id, supplier_id, live_supplier_invoice),
    KEY idx_purchases_supplier (tenant_id, supplier_id, purchase_date),
    KEY idx_purchases_date (tenant_id, branch_id, purchase_date),
    KEY idx_purchases_due (tenant_id, supplier_id, due_total),
    CONSTRAINT fk_purchases_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_purchases_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT fk_purchases_supplier FOREIGN KEY (tenant_id, supplier_id) REFERENCES suppliers (tenant_id, id),
    CONSTRAINT ck_purchases_status   CHECK (status IN ('draft','ordered','approved','partially_received','received','cancelled')),
    CONSTRAINT ck_purchases_pay      CHECK (payment_status IN ('unpaid','partial','paid','refunded')),
    CONSTRAINT ck_purchases_amounts  CHECK (subtotal >= 0 AND item_discount_total >= 0 AND bill_discount_amount >= 0
                                            AND tax_total >= 0 AND shipping_cost >= 0 AND other_cost >= 0
                                            AND grand_total >= 0 AND paid_total >= 0 AND returned_total >= 0 AND due_total >= 0)
);

CREATE TABLE purchase_items (
    tenant_id            BINARY(16)    NOT NULL,
    id                   BINARY(16)    NOT NULL,
    purchase_id          BINARY(16)    NOT NULL,
    line_no              SMALLINT      NOT NULL,
    variant_id           BINARY(16)    NOT NULL,
    product_name         VARCHAR(200)  NOT NULL,
    sku                  VARCHAR(64)   NOT NULL,
    unit_id              BINARY(16)    NOT NULL,
    unit_short_name      VARCHAR(10)   NOT NULL,
    unit_factor          DECIMAL(18,6) NOT NULL DEFAULT 1,
    quantity             DECIMAL(18,3) NOT NULL,
    base_quantity        DECIMAL(18,3) NOT NULL,
    received_quantity    DECIMAL(18,3) NOT NULL DEFAULT 0,
    returned_quantity    DECIMAL(18,3) NOT NULL DEFAULT 0,
    unit_cost            DECIMAL(18,4) NOT NULL,
    discount_type        VARCHAR(10),
    discount_value       DECIMAL(18,4),
    line_discount        DECIMAL(18,2) NOT NULL DEFAULT 0,
    bill_discount_alloc  DECIMAL(18,2) NOT NULL DEFAULT 0,
    tax_rate_pct         DECIMAL(7,4)  NOT NULL DEFAULT 0,
    tax_amount           DECIMAL(18,2) NOT NULL DEFAULT 0,
    charges_alloc        DECIMAL(18,2) NOT NULL DEFAULT 0,
    line_total           DECIMAL(18,2) NOT NULL,
    landed_unit_cost     DECIMAL(18,4) NOT NULL,
    previous_unit_cost   DECIMAL(18,4),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_purchase_items_line (tenant_id, purchase_id, line_no),
    KEY idx_purchase_items_variant (tenant_id, variant_id),
    CONSTRAINT fk_purchase_items_purchase FOREIGN KEY (tenant_id, purchase_id) REFERENCES purchases (tenant_id, id),
    CONSTRAINT fk_purchase_items_variant  FOREIGN KEY (tenant_id, variant_id)  REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_purchase_items_qty      CHECK (quantity > 0 AND base_quantity > 0 AND unit_factor > 0
                                                 AND received_quantity >= 0 AND received_quantity <= base_quantity
                                                 AND returned_quantity >= 0 AND returned_quantity <= received_quantity),
    CONSTRAINT ck_purchase_items_cost     CHECK (unit_cost >= 0),
    CONSTRAINT ck_purchase_items_disc     CHECK (discount_type IS NULL OR discount_type IN ('amount','percent'))
);

CREATE TABLE goods_receipts (
    tenant_id      BINARY(16)   NOT NULL,
    id             BINARY(16)   NOT NULL,
    receipt_no     VARCHAR(40)  NOT NULL,
    purchase_id    BINARY(16)   NOT NULL,
    location_id    BINARY(16)   NOT NULL,
    received_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    business_date  DATE         NOT NULL,
    note           VARCHAR(500),
    received_by    BINARY(16)   NOT NULL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_goods_receipts_no (tenant_id, receipt_no),
    KEY idx_grn_purchase (tenant_id, purchase_id),
    CONSTRAINT fk_grn_purchase FOREIGN KEY (tenant_id, purchase_id) REFERENCES purchases (tenant_id, id),
    CONSTRAINT fk_grn_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id)
);

CREATE TABLE goods_receipt_items (
    tenant_id         BINARY(16)    NOT NULL,
    id                BINARY(16)    NOT NULL,
    receipt_id        BINARY(16)    NOT NULL,
    purchase_item_id  BINARY(16)    NOT NULL,
    variant_id        BINARY(16)    NOT NULL,
    base_quantity     DECIMAL(18,3) NOT NULL,
    unit_cost         DECIMAL(18,4) NOT NULL,
    PRIMARY KEY (tenant_id, id),
    CONSTRAINT fk_grn_items_receipt FOREIGN KEY (tenant_id, receipt_id)       REFERENCES goods_receipts (tenant_id, id),
    CONSTRAINT fk_grn_items_pi      FOREIGN KEY (tenant_id, purchase_item_id) REFERENCES purchase_items (tenant_id, id),
    CONSTRAINT fk_grn_items_variant FOREIGN KEY (tenant_id, variant_id)       REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_grn_items_qty     CHECK (base_quantity > 0)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- purchases
CREATE TRIGGER trg_purchases_tg_ins BEFORE INSERT ON purchases FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchases_tg_upd BEFORE UPDATE ON purchases FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchases_tg_del BEFORE DELETE ON purchases FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- purchase_items
CREATE TRIGGER trg_purchase_items_tg_ins BEFORE INSERT ON purchase_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchase_items_tg_upd BEFORE UPDATE ON purchase_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchase_items_tg_del BEFORE DELETE ON purchase_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- goods_receipts
CREATE TRIGGER trg_goods_receipts_tg_ins BEFORE INSERT ON goods_receipts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_goods_receipts_tg_upd BEFORE UPDATE ON goods_receipts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_goods_receipts_tg_del BEFORE DELETE ON goods_receipts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- goods_receipt_items
CREATE TRIGGER trg_goods_receipt_items_tg_ins BEFORE INSERT ON goods_receipt_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_goods_receipt_items_tg_upd BEFORE UPDATE ON goods_receipt_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_goods_receipt_items_tg_del BEFORE DELETE ON goods_receipt_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
