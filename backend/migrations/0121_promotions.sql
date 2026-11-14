-- 0121 discounts and promotions (Final_docs/05_discount-system.md): product
-- discounts (percent / fixed amount / fixed price), flash sales with a
-- quantity limit, Buy X Get Y, coupons, and what each sale actually got.
--
-- Schedules are stored in the business's local time (the doc's "Asia/Dhaka"
-- rule) and checked against the server clock in that time zone.
-- `status` is what the owner set (draft / active / paused / ended); whether
-- an active promotion is "scheduled", "running" or "expired" follows from
-- its dates, so it is always right at the moment of sale.

CREATE TABLE promotions (
    tenant_id                BINARY(16)    NOT NULL,
    id                       BINARY(16)    NOT NULL,
    name                     VARCHAR(120)  NOT NULL,
    name_bn                  VARCHAR(120)  NULL,
    type                     VARCHAR(20)   NOT NULL,
    discount_type            VARCHAR(20)   NOT NULL,
    discount_value           DECIMAL(18,4) NOT NULL DEFAULT 0,
    -- Cap of what the promotion gives per invoice.
    max_discount_amount      DECIMAL(18,2) NULL,
    -- Gross cart subtotal required (mostly coupons).
    min_purchase_amount      DECIMAL(18,2) NULL,
    starts_at                DATETIME      NULL,
    ends_at                  DATETIME      NULL,
    -- e.g. ["fri","sat"]; NULL = every day.
    active_days              JSON          NULL,
    active_time_from         TIME          NULL,
    active_time_to           TIME          NULL,
    priority                 INT           NOT NULL DEFAULT 0,
    stackable                BOOLEAN       NOT NULL DEFAULT FALSE,
    applies_to_customer      VARCHAR(20)   NOT NULL DEFAULT 'all',
    total_usage_limit        INT           NULL,
    usage_count              INT           NOT NULL DEFAULT 0,
    per_customer_limit       INT           NULL,
    -- Flash: discounted units allowed / sold.
    quantity_limit           DECIMAL(18,3) NULL,
    quantity_used            DECIMAL(18,3) NOT NULL DEFAULT 0,
    -- Buy X Get Y (one rule per promotion).
    bxgy_buy_quantity        DECIMAL(18,3) NULL,
    bxgy_get_quantity        DECIMAL(18,3) NULL,
    bxgy_get_target_type     VARCHAR(20)   NULL,
    bxgy_get_target_id       BINARY(16)    NULL,
    bxgy_get_discount_type   VARCHAR(20)   NULL,
    bxgy_get_discount_value  DECIMAL(18,4) NULL,
    bxgy_repeat              BOOLEAN       NOT NULL DEFAULT TRUE,
    bxgy_auto_add            BOOLEAN       NOT NULL DEFAULT FALSE,
    status                   VARCHAR(20)   NOT NULL DEFAULT 'draft',
    -- Set by the first sale that used it: benefit and products are then locked.
    first_used_at            DATETIME      NULL,
    version                  INT           NOT NULL DEFAULT 1,
    created_by               BINARY(16)    NULL,
    updated_by               BINARY(16)    NULL,
    created_at               DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    deleted_at               DATETIME      NULL,
    PRIMARY KEY (tenant_id, id),
    KEY idx_promotions_status (tenant_id, status, deleted_at),
    CONSTRAINT ck_promotions_type     CHECK (type IN ('product_discount','flash','bxgy','coupon')),
    CONSTRAINT ck_promotions_dtype    CHECK (discount_type IN ('percent','fixed_amount','fixed_price','free')),
    CONSTRAINT ck_promotions_status   CHECK (status IN ('draft','active','paused','ended')),
    CONSTRAINT ck_promotions_customer CHECK (applies_to_customer IN ('all','registered_only')),
    CONSTRAINT ck_promotions_amounts  CHECK (discount_value >= 0 AND (max_discount_amount IS NULL OR max_discount_amount >= 0)
                                             AND (min_purchase_amount IS NULL OR min_purchase_amount >= 0)
                                             AND usage_count >= 0 AND quantity_used >= 0),
    CONSTRAINT ck_promotions_dates    CHECK (starts_at IS NULL OR ends_at IS NULL OR ends_at > starts_at),
    CONSTRAINT ck_promotions_bxgy     CHECK (bxgy_get_target_type IS NULL OR bxgy_get_target_type IN ('same_product','product','category'))
);

-- Which products qualify: products, categories (with their sub-categories),
-- brands or everything, minus exclusions.
CREATE TABLE promotion_targets (
    tenant_id     BINARY(16)  NOT NULL,
    id            BINARY(16)  NOT NULL,
    promotion_id  BINARY(16)  NOT NULL,
    target_type   VARCHAR(20) NOT NULL,
    target_id     BINARY(16)  NULL,
    is_exclusion  BOOLEAN     NOT NULL DEFAULT FALSE,
    PRIMARY KEY (tenant_id, id),
    KEY idx_promotion_targets (tenant_id, promotion_id),
    CONSTRAINT fk_promotion_targets_promo FOREIGN KEY (tenant_id, promotion_id) REFERENCES promotions (tenant_id, id),
    CONSTRAINT ck_promotion_targets_type  CHECK (target_type IN ('product','category','brand','all_products')),
    CONSTRAINT ck_promotion_targets_id    CHECK ((target_type = 'all_products') = (target_id IS NULL))
);

-- No rows = every branch.
CREATE TABLE promotion_branches (
    tenant_id     BINARY(16) NOT NULL,
    promotion_id  BINARY(16) NOT NULL,
    branch_id     BINARY(16) NOT NULL,
    PRIMARY KEY (tenant_id, promotion_id, branch_id),
    CONSTRAINT fk_promotion_branches_promo  FOREIGN KEY (tenant_id, promotion_id) REFERENCES promotions (tenant_id, id),
    CONSTRAINT fk_promotion_branches_branch FOREIGN KEY (tenant_id, branch_id)    REFERENCES branches (tenant_id, id)
);

-- Codes of a coupon promotion: one shared public code or generated single-use codes.
CREATE TABLE coupons (
    tenant_id             BINARY(16)  NOT NULL,
    id                    BINARY(16)  NOT NULL,
    promotion_id          BINARY(16)  NOT NULL,
    code                  VARCHAR(40) NOT NULL,
    type                  VARCHAR(20) NOT NULL DEFAULT 'public',
    usage_limit           INT         NULL,
    used_count            INT         NOT NULL DEFAULT 0,
    assigned_customer_id  BINARY(16)  NULL,
    expires_at            DATETIME    NULL,
    status                VARCHAR(20) NOT NULL DEFAULT 'active',
    created_by            BINARY(16)  NULL,
    created_at            DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, id),
    -- The collation is case-insensitive, so EID20 and eid20 are the same code.
    UNIQUE KEY uq_coupons_code (tenant_id, code),
    KEY idx_coupons_promotion (tenant_id, promotion_id),
    CONSTRAINT fk_coupons_promo    FOREIGN KEY (tenant_id, promotion_id)         REFERENCES promotions (tenant_id, id),
    CONSTRAINT fk_coupons_customer FOREIGN KEY (tenant_id, assigned_customer_id) REFERENCES customers (tenant_id, id),
    CONSTRAINT ck_coupons_type     CHECK (type IN ('public','unique')),
    CONSTRAINT ck_coupons_status   CHECK (status IN ('active','used','disabled')),
    CONSTRAINT ck_coupons_counts   CHECK (used_count >= 0 AND (usage_limit IS NULL OR usage_limit > 0))
);

CREATE TABLE coupon_redemptions (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    coupon_id        BINARY(16)    NOT NULL,
    promotion_id     BINARY(16)    NOT NULL,
    sale_id          BINARY(16)    NOT NULL,
    customer_id      BINARY(16)    NOT NULL,
    branch_id        BINARY(16)    NOT NULL,
    user_id          BINARY(16)    NOT NULL,
    discount_amount  DECIMAL(18,2) NOT NULL,
    status           VARCHAR(20)   NOT NULL DEFAULT 'applied',
    created_at       DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reversed_at      DATETIME      NULL,
    PRIMARY KEY (tenant_id, id),
    KEY idx_coupon_redemptions_coupon   (tenant_id, coupon_id),
    KEY idx_coupon_redemptions_customer (tenant_id, customer_id, promotion_id),
    KEY idx_coupon_redemptions_sale     (tenant_id, sale_id),
    CONSTRAINT fk_coupon_redemptions_coupon FOREIGN KEY (tenant_id, coupon_id) REFERENCES coupons (tenant_id, id),
    CONSTRAINT fk_coupon_redemptions_sale   FOREIGN KEY (tenant_id, sale_id)   REFERENCES sales (tenant_id, id),
    CONSTRAINT ck_coupon_redemptions_status CHECK (status IN ('applied','reversed'))
);

-- What each sale line actually got. Old sales are never recalculated from
-- current promotions: names, values and Buy X Get Y rules are snapshots.
CREATE TABLE sale_item_discounts (
    tenant_id                BINARY(16)    NOT NULL,
    id                       BINARY(16)    NOT NULL,
    sale_id                  BINARY(16)    NOT NULL,
    sale_item_id             BINARY(16)    NOT NULL,
    promotion_id             BINARY(16)    NOT NULL,
    coupon_id                BINARY(16)    NULL,
    coupon_code              VARCHAR(40)   NULL,
    promotion_type           VARCHAR(20)   NOT NULL,
    promotion_name_snapshot  VARCHAR(120)  NOT NULL,
    promotion_name_bn        VARCHAR(120)  NULL,
    discount_type            VARCHAR(20)   NOT NULL,
    discount_value           DECIMAL(18,4) NOT NULL,
    discount_amount          DECIMAL(18,2) NOT NULL,
    units                    DECIMAL(18,3) NOT NULL DEFAULT 0,
    free_quantity            DECIMAL(18,3) NOT NULL DEFAULT 0,
    bxgy_buy_quantity        DECIMAL(18,3) NULL,
    bxgy_get_quantity        DECIMAL(18,3) NULL,
    bxgy_get_target_type     VARCHAR(20)   NULL,
    created_at               DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, id),
    KEY idx_sale_item_discounts_sale  (tenant_id, sale_id),
    KEY idx_sale_item_discounts_item  (tenant_id, sale_item_id),
    KEY idx_sale_item_discounts_promo (tenant_id, promotion_id, created_at),
    CONSTRAINT fk_sale_item_discounts_sale FOREIGN KEY (tenant_id, sale_id)      REFERENCES sales (tenant_id, id),
    CONSTRAINT fk_sale_item_discounts_item FOREIGN KEY (tenant_id, sale_item_id) REFERENCES sale_items (tenant_id, id),
    CONSTRAINT ck_sale_item_discounts_amt  CHECK (discount_amount >= 0 AND units >= 0 AND free_quantity >= 0)
);

ALTER TABLE sales
    ADD COLUMN promotion_discount_total DECIMAL(18,2) NOT NULL DEFAULT 0 AFTER item_discount_total,
    ADD COLUMN coupon_discount_total    DECIMAL(18,2) NOT NULL DEFAULT 0 AFTER promotion_discount_total,
    ADD COLUMN coupon_code              VARCHAR(40)   NULL AFTER coupon_discount_total;

ALTER TABLE sale_items
    ADD COLUMN promotion_discount DECIMAL(18,2) NOT NULL DEFAULT 0 AFTER gross_amount,
    ADD COLUMN free_quantity      DECIMAL(18,3) NOT NULL DEFAULT 0 AFTER promotion_discount;

-- Settings → Discounts (§5 defaults).
ALTER TABLE tenant_settings
    ADD COLUMN promotions_stack                 BOOLEAN NOT NULL DEFAULT FALSE AFTER show_product_images,
    ADD COLUMN coupon_stacks                    BOOLEAN NOT NULL DEFAULT FALSE AFTER promotions_stack,
    ADD COLUMN manual_discount_with_promotions  BOOLEAN NOT NULL DEFAULT TRUE  AFTER coupon_stacks,
    ADD COLUMN restore_coupon_on_full_return    BOOLEAN NOT NULL DEFAULT FALSE AFTER manual_discount_with_promotions;

INSERT INTO permissions (code, module, description, is_sensitive) VALUES
 ('promotions.view',      'promotions', 'View promotions, coupons and their reports', FALSE),
 ('promotions.manage',    'promotions', 'Create, edit, pause and end promotions; generate coupons', TRUE),
 ('pos.apply_coupon',     'pos',        'Apply a coupon code at the POS', FALSE),
 ('pos.remove_promotion', 'pos',        'Remove an automatic promotion from a cart line', TRUE);

-- Existing businesses: the system roles that get them for new businesses.
SET @app_platform = 1;

INSERT IGNORE INTO role_permissions (tenant_id, role_id, permission_code)
SELECT r.tenant_id, r.id, p.code
FROM roles r
JOIN permissions p ON p.code IN ('promotions.view', 'promotions.manage', 'pos.apply_coupon', 'pos.remove_promotion')
WHERE r.is_system AND r.code IN ('owner', 'admin', 'branch_manager');

INSERT IGNORE INTO role_permissions (tenant_id, role_id, permission_code)
SELECT r.tenant_id, r.id, 'pos.apply_coupon'
FROM roles r
WHERE r.is_system AND r.code IN ('cashier', 'salesperson');

INSERT IGNORE INTO role_permissions (tenant_id, role_id, permission_code)
SELECT r.tenant_id, r.id, 'promotions.view'
FROM roles r
WHERE r.is_system AND r.code IN ('accountant');

SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- promotions
CREATE TRIGGER trg_promotions_tg_ins BEFORE INSERT ON promotions FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_promotions_tg_upd BEFORE UPDATE ON promotions FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_promotions_tg_del BEFORE DELETE ON promotions FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- promotion_targets
CREATE TRIGGER trg_promotion_targets_tg_ins BEFORE INSERT ON promotion_targets FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_promotion_targets_tg_upd BEFORE UPDATE ON promotion_targets FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_promotion_targets_tg_del BEFORE DELETE ON promotion_targets FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- promotion_branches
CREATE TRIGGER trg_promotion_branches_tg_ins BEFORE INSERT ON promotion_branches FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_promotion_branches_tg_upd BEFORE UPDATE ON promotion_branches FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_promotion_branches_tg_del BEFORE DELETE ON promotion_branches FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- coupons
CREATE TRIGGER trg_coupons_tg_ins BEFORE INSERT ON coupons FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_coupons_tg_upd BEFORE UPDATE ON coupons FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_coupons_tg_del BEFORE DELETE ON coupons FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- coupon_redemptions
CREATE TRIGGER trg_coupon_redemptions_tg_ins BEFORE INSERT ON coupon_redemptions FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_coupon_redemptions_tg_upd BEFORE UPDATE ON coupon_redemptions FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_coupon_redemptions_tg_del BEFORE DELETE ON coupon_redemptions FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- sale_item_discounts
CREATE TRIGGER trg_sale_item_discounts_tg_ins BEFORE INSERT ON sale_item_discounts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_item_discounts_tg_upd BEFORE UPDATE ON sale_item_discounts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_sale_item_discounts_tg_del BEFORE DELETE ON sale_item_discounts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
