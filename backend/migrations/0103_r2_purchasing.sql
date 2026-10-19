-- 0103 purchase returns (DB design §9.13, PUR-R07). Stock leaves at the
-- current WAC; the supplier is debited the original line price (§8.5).

CREATE TABLE purchase_returns (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    branch_id        BINARY(16)    NOT NULL,
    return_no        VARCHAR(40)   NOT NULL,
    purchase_id      BINARY(16)    NOT NULL,
    supplier_id      BINARY(16)    NOT NULL,
    location_id      BINARY(16)    NOT NULL,
    business_date    DATE          NOT NULL,
    reason           VARCHAR(500)  NOT NULL,
    total            DECIMAL(18,2) NOT NULL,
    tax_total        DECIMAL(18,2) NOT NULL DEFAULT 0,
    cost_total       DECIMAL(18,2) NOT NULL DEFAULT 0,
    settlement_mode  VARCHAR(20)   NOT NULL,
    applied_to_due   DECIMAL(18,2) NOT NULL DEFAULT 0,
    refunded_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    credited_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    idempotency_key  BINARY(16)    NOT NULL,
    created_by       BINARY(16)    NOT NULL,
    created_at       DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_purchase_returns_no (tenant_id, branch_id, return_no),
    UNIQUE KEY uq_purchase_returns_idem (tenant_id, idempotency_key),
    KEY idx_purchase_returns_purchase (tenant_id, purchase_id),
    KEY idx_purchase_returns_date (tenant_id, business_date),
    CONSTRAINT fk_pr_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_pr_purchase FOREIGN KEY (tenant_id, purchase_id) REFERENCES purchases (tenant_id, id),
    CONSTRAINT fk_pr_supplier FOREIGN KEY (tenant_id, supplier_id) REFERENCES suppliers (tenant_id, id),
    CONSTRAINT fk_pr_location FOREIGN KEY (tenant_id, location_id) REFERENCES stock_locations (tenant_id, id),
    CONSTRAINT ck_pr_total    CHECK (total >= 0 AND tax_total >= 0 AND cost_total >= 0),
    CONSTRAINT ck_pr_mode     CHECK (settlement_mode IN ('reduce_payable','refund')),
    CONSTRAINT ck_pr_split    CHECK (applied_to_due + refunded_amount + credited_amount = total)
);

CREATE TABLE purchase_return_items (
    tenant_id         BINARY(16)    NOT NULL,
    id                BINARY(16)    NOT NULL,
    return_id         BINARY(16)    NOT NULL,
    purchase_item_id  BINARY(16)    NOT NULL,
    variant_id        BINARY(16)    NOT NULL,
    base_quantity     DECIMAL(18,3) NOT NULL,
    tax_amount        DECIMAL(18,2) NOT NULL DEFAULT 0,
    line_total        DECIMAL(18,2) NOT NULL,
    unit_cost_out     DECIMAL(18,4) NOT NULL,
    PRIMARY KEY (tenant_id, id),
    KEY idx_pri_return (tenant_id, return_id),
    CONSTRAINT fk_pri_return  FOREIGN KEY (tenant_id, return_id)        REFERENCES purchase_returns (tenant_id, id),
    CONSTRAINT fk_pri_pi      FOREIGN KEY (tenant_id, purchase_item_id) REFERENCES purchase_items (tenant_id, id),
    CONSTRAINT fk_pri_variant FOREIGN KEY (tenant_id, variant_id)       REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_pri_qty     CHECK (base_quantity > 0)
);

-- Return numbers for businesses created before this migration.
SET @app_platform = 1;

INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding)
SELECT b.tenant_id, UUID_TO_BIN(UUID(), 1), b.id, 'purchase_return', 'PRT', TRUE, 5
FROM branches b
WHERE NOT EXISTS (SELECT 1 FROM document_sequences s
                  WHERE s.tenant_id = b.tenant_id AND s.branch_id = b.id AND s.doc_type = 'purchase_return');

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- purchase_returns
CREATE TRIGGER trg_purchase_returns_tg_ins BEFORE INSERT ON purchase_returns FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchase_returns_tg_upd BEFORE UPDATE ON purchase_returns FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchase_returns_tg_del BEFORE DELETE ON purchase_returns FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- purchase_return_items
CREATE TRIGGER trg_purchase_return_items_tg_ins BEFORE INSERT ON purchase_return_items FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchase_return_items_tg_upd BEFORE UPDATE ON purchase_return_items FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_purchase_return_items_tg_del BEFORE DELETE ON purchase_return_items FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
