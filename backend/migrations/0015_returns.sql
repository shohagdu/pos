-- 0015 sale returns (DB design §9.12, Requirement Analysis §7.11). Returned
-- quantities are checked against sale_items.returned_quantity under a row lock.

CREATE TABLE sale_returns (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    branch_id        BINARY(16)    NOT NULL,
    return_no        VARCHAR(40)   NOT NULL,
    sale_id          BINARY(16)    NOT NULL,
    customer_id      BINARY(16)    NOT NULL,
    return_date      DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    business_date    DATE          NOT NULL,
    reason           VARCHAR(500)  NOT NULL,
    refund_mode      VARCHAR(20)   NOT NULL,
    subtotal         DECIMAL(18,2) NOT NULL,
    tax_total        DECIMAL(18,2) NOT NULL DEFAULT 0,
    total            DECIMAL(18,2) NOT NULL,
    applied_to_due   DECIMAL(18,2) NOT NULL DEFAULT 0,
    refunded_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    credited_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    cost_total       DECIMAL(18,2) NOT NULL DEFAULT 0,
    status           VARCHAR(20)   NOT NULL DEFAULT 'posted',
    approved_by      BINARY(16),
    idempotency_key  BINARY(16)    NOT NULL,
    created_by       BINARY(16)    NOT NULL,
    created_at       DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_sale_returns_no (tenant_id, branch_id, return_no),
    UNIQUE KEY uq_sale_returns_idem (tenant_id, idempotency_key),
    KEY idx_sale_returns_sale (tenant_id, sale_id),
    KEY idx_sale_returns_date (tenant_id, branch_id, business_date),
    CONSTRAINT fk_sale_returns_sale     FOREIGN KEY (tenant_id, sale_id)     REFERENCES sales (tenant_id, id),
    CONSTRAINT fk_sale_returns_customer FOREIGN KEY (tenant_id, customer_id) REFERENCES customers (tenant_id, id),
    CONSTRAINT fk_sale_returns_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT ck_sale_returns_mode     CHECK (refund_mode IN ('cash','original_method','customer_credit')),
    CONSTRAINT ck_sale_returns_status   CHECK (status IN ('pending_approval','posted','rejected')),
    CONSTRAINT ck_sale_returns_amounts  CHECK (subtotal >= 0 AND tax_total >= 0 AND total >= 0 AND applied_to_due >= 0
                                               AND refunded_amount >= 0 AND credited_amount >= 0),
    CONSTRAINT ck_sale_returns_split    CHECK (applied_to_due + refunded_amount + credited_amount = total)
);

CREATE TABLE sale_return_items (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    return_id        BINARY(16)    NOT NULL,
    sale_item_id     BINARY(16)    NOT NULL,
    variant_id       BINARY(16)    NOT NULL,
    quantity         DECIMAL(18,3) NOT NULL,
    base_quantity    DECIMAL(18,3) NOT NULL,
    item_condition   VARCHAR(10)   NOT NULL,
    location_id      BINARY(16)    NOT NULL,
    taxable_amount   DECIMAL(18,2) NOT NULL,
    tax_amount       DECIMAL(18,2) NOT NULL DEFAULT 0,
    line_total       DECIMAL(18,2) NOT NULL,
    unit_cost        DECIMAL(18,4) NOT NULL,
    PRIMARY KEY (tenant_id, id),
    KEY idx_sri_return (tenant_id, return_id),
    CONSTRAINT fk_sri_return    FOREIGN KEY (tenant_id, return_id)    REFERENCES sale_returns (tenant_id, id),
    CONSTRAINT fk_sri_sale_item FOREIGN KEY (tenant_id, sale_item_id) REFERENCES sale_items (tenant_id, id),
    CONSTRAINT fk_sri_variant   FOREIGN KEY (tenant_id, variant_id)   REFERENCES product_variants (tenant_id, id),
    CONSTRAINT fk_sri_location  FOREIGN KEY (tenant_id, location_id)  REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT ck_sri_qty       CHECK (quantity > 0 AND base_quantity > 0),
    CONSTRAINT ck_sri_condition CHECK (item_condition IN ('restock','damaged'))
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- sale_returns
CREATE TRIGGER trg_sale_returns_tg_ins BEFORE INSERT ON sale_returns FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_returns_tg_upd BEFORE UPDATE ON sale_returns FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_returns_tg_del BEFORE DELETE ON sale_returns FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- sale_return_items
CREATE TRIGGER trg_sale_return_items_tg_ins BEFORE INSERT ON sale_return_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_return_items_tg_upd BEFORE UPDATE ON sale_return_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_return_items_tg_del BEFORE DELETE ON sale_return_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
