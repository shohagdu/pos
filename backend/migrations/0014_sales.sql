-- 0014 sales: invoices with line snapshots and held carts (DB design §9.12).
-- A sale is posted atomically by pos_sales (DB design §10.1); there is no
-- persisted draft. Registers and shifts are Release 2.

CREATE TABLE sales (
    tenant_id                BINARY(16)    NOT NULL,
    id                       BINARY(16)    NOT NULL,
    branch_id                BINARY(16)    NOT NULL,
    location_id              BINARY(16)    NOT NULL,
    invoice_no               VARCHAR(40)   NOT NULL,
    sale_date                DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    business_date            DATE          NOT NULL,
    customer_id              BINARY(16)    NOT NULL,
    cashier_id               BINARY(16)    NOT NULL,
    salesperson_id           BINARY(16),
    register_shift_id        BINARY(16),
    price_list_id            BINARY(16),
    status                   VARCHAR(20)   NOT NULL DEFAULT 'completed',
    payment_status           VARCHAR(10)   NOT NULL,
    prices_include_tax       BOOLEAN       NOT NULL,
    subtotal                 DECIMAL(18,2) NOT NULL,
    item_discount_total      DECIMAL(18,2) NOT NULL DEFAULT 0,
    invoice_discount_type    VARCHAR(10),
    invoice_discount_value   DECIMAL(18,4),
    invoice_discount_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    tax_total                DECIMAL(18,2) NOT NULL DEFAULT 0,
    shipping_charge          DECIMAL(18,2) NOT NULL DEFAULT 0,
    other_charge             DECIMAL(18,2) NOT NULL DEFAULT 0,
    rounding_adjustment      DECIMAL(18,2) NOT NULL DEFAULT 0,
    grand_total              DECIMAL(18,2) NOT NULL,
    paid_total               DECIMAL(18,2) NOT NULL DEFAULT 0,
    returned_total           DECIMAL(18,2) NOT NULL DEFAULT 0,
    due_total                DECIMAL(18,2) NOT NULL DEFAULT 0,
    change_given             DECIMAL(18,2) NOT NULL DEFAULT 0,
    cost_total               DECIMAL(18,2) NOT NULL DEFAULT 0,
    note                     VARCHAR(500),
    reprint_count            INT           NOT NULL DEFAULT 0,
    idempotency_key          BINARY(16)    NOT NULL,
    cancelled_at             DATETIME(6),
    cancelled_by             BINARY(16),
    cancel_reason            VARCHAR(500),
    created_at               DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at               DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_sales_invoice (tenant_id, branch_id, invoice_no),
    UNIQUE KEY uq_sales_idempotency (tenant_id, idempotency_key),
    KEY idx_sales_branch_date (tenant_id, branch_id, business_date),
    KEY idx_sales_date (tenant_id, business_date, sale_date),
    KEY idx_sales_customer (tenant_id, customer_id, sale_date),
    KEY idx_sales_cashier (tenant_id, cashier_id, business_date),
    KEY idx_sales_due (tenant_id, customer_id, due_total),
    CONSTRAINT fk_sales_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_sales_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT fk_sales_customer FOREIGN KEY (tenant_id, customer_id) REFERENCES customers (tenant_id, id),
    CONSTRAINT ck_sales_status   CHECK (status IN ('completed','partially_returned','returned','cancelled')),
    CONSTRAINT ck_sales_pay      CHECK (payment_status IN ('unpaid','partial','paid','refunded')),
    CONSTRAINT ck_sales_disc     CHECK (invoice_discount_type IS NULL OR invoice_discount_type IN ('amount','percent')),
    CONSTRAINT ck_sales_amounts  CHECK (subtotal >= 0 AND item_discount_total >= 0 AND invoice_discount_amount >= 0
                                        AND tax_total >= 0 AND shipping_charge >= 0 AND other_charge >= 0
                                        AND grand_total >= 0 AND paid_total >= 0 AND returned_total >= 0
                                        AND due_total >= 0 AND change_given >= 0),
    CONSTRAINT ck_sales_bounds   CHECK (due_total <= grand_total AND paid_total <= grand_total AND returned_total <= grand_total),
    CONSTRAINT ck_sales_cancel   CHECK (status <> 'cancelled' OR (cancelled_at IS NOT NULL AND cancel_reason IS NOT NULL))
);

CREATE TABLE sale_items (
    tenant_id               BINARY(16)    NOT NULL,
    id                      BINARY(16)    NOT NULL,
    sale_id                 BINARY(16)    NOT NULL,
    line_no                 SMALLINT      NOT NULL,
    variant_id              BINARY(16)    NOT NULL,
    product_name            VARCHAR(200)  NOT NULL,
    variant_name            VARCHAR(100),
    sku                     VARCHAR(64)   NOT NULL,
    unit_id                 BINARY(16)    NOT NULL,
    unit_short_name         VARCHAR(10)   NOT NULL,
    unit_factor             DECIMAL(18,6) NOT NULL DEFAULT 1,
    track_inventory         BOOLEAN       NOT NULL DEFAULT TRUE,
    quantity                DECIMAL(18,3) NOT NULL,
    base_quantity           DECIMAL(18,3) NOT NULL,
    returned_quantity       DECIMAL(18,3) NOT NULL DEFAULT 0,
    list_price              DECIMAL(18,4) NOT NULL,
    unit_price              DECIMAL(18,4) NOT NULL,
    price_overridden        BOOLEAN       NOT NULL DEFAULT FALSE,
    gross_amount            DECIMAL(18,2) NOT NULL,
    discount_type           VARCHAR(10),
    discount_value          DECIMAL(18,4),
    line_discount           DECIMAL(18,2) NOT NULL DEFAULT 0,
    invoice_discount_alloc  DECIMAL(18,2) NOT NULL DEFAULT 0,
    taxable_amount          DECIMAL(18,2) NOT NULL,
    tax_rate_id             BINARY(16),
    tax_rate_pct            DECIMAL(7,4)  NOT NULL DEFAULT 0,
    tax_amount              DECIMAL(18,2) NOT NULL DEFAULT 0,
    line_total              DECIMAL(18,2) NOT NULL,
    unit_cost               DECIMAL(18,4) NOT NULL DEFAULT 0,
    cost_total              DECIMAL(18,2) NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_sale_items_line (tenant_id, sale_id, line_no),
    KEY idx_sale_items_variant (tenant_id, variant_id),
    CONSTRAINT fk_sale_items_sale    FOREIGN KEY (tenant_id, sale_id)    REFERENCES sales (tenant_id, id),
    CONSTRAINT fk_sale_items_variant FOREIGN KEY (tenant_id, variant_id) REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_sale_items_qty     CHECK (quantity > 0 AND base_quantity > 0 AND unit_factor > 0
                                            AND returned_quantity >= 0 AND returned_quantity <= quantity),
    CONSTRAINT ck_sale_items_amounts CHECK (list_price >= 0 AND unit_price >= 0 AND line_discount >= 0
                                            AND invoice_discount_alloc >= 0 AND taxable_amount >= 0
                                            AND tax_amount >= 0 AND line_total >= 0),
    CONSTRAINT ck_sale_items_disc    CHECK (discount_type IS NULL OR discount_type IN ('amount','percent'))
);

CREATE TABLE held_carts (
    tenant_id    BINARY(16)   NOT NULL,
    id           BINARY(16)   NOT NULL,
    branch_id    BINARY(16)   NOT NULL,
    label        VARCHAR(100),
    customer_id  BINARY(16),
    cart         JSON         NOT NULL,
    held_by      BINARY(16)   NOT NULL,
    expires_at   DATETIME(6)  NOT NULL,
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_held_carts_branch (tenant_id, branch_id, created_at),
    CONSTRAINT fk_held_carts_branch FOREIGN KEY (tenant_id, branch_id) REFERENCES branches (tenant_id, id)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- sales
CREATE TRIGGER trg_sales_tg_ins BEFORE INSERT ON sales FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sales_tg_upd BEFORE UPDATE ON sales FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sales_tg_del BEFORE DELETE ON sales FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- sale_items
CREATE TRIGGER trg_sale_items_tg_ins BEFORE INSERT ON sale_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_items_tg_upd BEFORE UPDATE ON sale_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_items_tg_del BEFORE DELETE ON sale_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- held_carts
CREATE TRIGGER trg_held_carts_tg_ins BEFORE INSERT ON held_carts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_held_carts_tg_upd BEFORE UPDATE ON held_carts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_held_carts_tg_del BEFORE DELETE ON held_carts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
