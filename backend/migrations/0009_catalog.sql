-- 0009 catalogue: tax rates, units, categories, brands, products, variants,
-- barcodes, price history (DB design §9.7). Alternative units and price lists
-- are Release 2. product image files arrive with 0011 (files); image_file_id
-- gets its foreign key then.

CREATE TABLE tax_rates (
    tenant_id   BINARY(16)   NOT NULL,
    id          BINARY(16)   NOT NULL,
    name        VARCHAR(50)  NOT NULL,
    rate_pct    DECIMAL(7,4) NOT NULL,
    is_default  BOOLEAN      NOT NULL DEFAULT FALSE,
    status      VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_tax_rates_name (tenant_id, name),
    CONSTRAINT fk_tax_rates_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_tax_rates_rate   CHECK (rate_pct BETWEEN 0 AND 100),
    CONSTRAINT ck_tax_rates_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE units (
    tenant_id       BINARY(16)  NOT NULL,
    id              BINARY(16)  NOT NULL,
    name            VARCHAR(50) NOT NULL,
    short_name      VARCHAR(10) NOT NULL,
    allow_fraction  BOOLEAN     NOT NULL DEFAULT FALSE,
    status          VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_units_short (tenant_id, short_name),
    CONSTRAINT fk_units_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_units_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE categories (
    tenant_id   BINARY(16)   NOT NULL,
    id          BINARY(16)   NOT NULL,
    parent_id   BINARY(16),
    parent_key  BINARY(16)   AS (COALESCE(parent_id, 0x00000000000000000000000000000000)) VIRTUAL,
    name        VARCHAR(100) NOT NULL,
    depth       TINYINT      NOT NULL DEFAULT 0,
    sort_order  INT          NOT NULL DEFAULT 0,
    status      VARCHAR(20)  NOT NULL DEFAULT 'active',
    version     INT          NOT NULL DEFAULT 1,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    deleted_at  DATETIME(6),
    live_name   VARCHAR(100) AS (IF(deleted_at IS NULL, name, NULL)) VIRTUAL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_categories_name (tenant_id, parent_key, live_name),
    CONSTRAINT fk_categories_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_categories_parent FOREIGN KEY (tenant_id, parent_id) REFERENCES categories (tenant_id, id),
    CONSTRAINT ck_categories_depth  CHECK (depth BETWEEN 0 AND 2),
    CONSTRAINT ck_categories_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE brands (
    tenant_id   BINARY(16)   NOT NULL,
    id          BINARY(16)   NOT NULL,
    name        VARCHAR(100) NOT NULL,
    status      VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    deleted_at  DATETIME(6),
    live_name   VARCHAR(100) AS (IF(deleted_at IS NULL, name, NULL)) VIRTUAL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_brands_name (tenant_id, live_name),
    CONSTRAINT fk_brands_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_brands_status CHECK (status IN ('active','inactive'))
);

CREATE TABLE products (
    tenant_id        BINARY(16)   NOT NULL,
    id               BINARY(16)   NOT NULL,
    product_type     VARCHAR(20)  NOT NULL DEFAULT 'standard',
    name             VARCHAR(200) NOT NULL,
    name_bn          VARCHAR(200),
    search_name      VARCHAR(400) NOT NULL,
    description      TEXT,
    category_id      BINARY(16),
    brand_id         BINARY(16),
    base_unit_id     BINARY(16)   NOT NULL,
    tax_rate_id      BINARY(16),
    track_inventory  BOOLEAN      NOT NULL DEFAULT TRUE,
    has_variants     BOOLEAN      NOT NULL DEFAULT FALSE,
    image_file_id    BINARY(16),
    attributes       JSON         NOT NULL DEFAULT (JSON_OBJECT()),
    status           VARCHAR(20)  NOT NULL DEFAULT 'active',
    is_sample        BOOLEAN      NOT NULL DEFAULT FALSE,
    version          INT          NOT NULL DEFAULT 1,
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    created_by       BINARY(16),
    updated_by       BINARY(16),
    deleted_at       DATETIME(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_products_category (tenant_id, category_id),
    KEY idx_products_name (tenant_id, name),
    CONSTRAINT fk_products_tenant   FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_products_category FOREIGN KEY (tenant_id, category_id)  REFERENCES categories (tenant_id, id),
    CONSTRAINT fk_products_brand    FOREIGN KEY (tenant_id, brand_id)     REFERENCES brands (tenant_id, id),
    CONSTRAINT fk_products_unit     FOREIGN KEY (tenant_id, base_unit_id) REFERENCES units (tenant_id, id),
    CONSTRAINT fk_products_tax      FOREIGN KEY (tenant_id, tax_rate_id)  REFERENCES tax_rates (tenant_id, id),
    CONSTRAINT ck_products_type     CHECK (product_type IN ('standard','service','combo')),
    CONSTRAINT ck_products_status   CHECK (status IN ('active','archived')),
    CONSTRAINT ck_products_service  CHECK (product_type <> 'service' OR track_inventory = FALSE)
);

CREATE TABLE product_variants (
    tenant_id              BINARY(16)    NOT NULL,
    id                     BINARY(16)    NOT NULL,
    product_id             BINARY(16)    NOT NULL,
    sku                    VARCHAR(64)   NOT NULL,
    variant_name           VARCHAR(100),
    option_values          JSON          NOT NULL DEFAULT (JSON_OBJECT()),
    is_default             BOOLEAN       NOT NULL DEFAULT FALSE,
    purchase_price         DECIMAL(18,4) NOT NULL DEFAULT 0,
    selling_price          DECIMAL(18,4) NOT NULL DEFAULT 0,
    wholesale_price        DECIMAL(18,4),
    min_selling_price      DECIMAL(18,4),
    reorder_level          DECIMAL(18,3),
    search_text            VARCHAR(500)  NOT NULL,
    last_purchase_price    DECIMAL(18,4),
    last_purchase_date     DATE,
    last_purchase_qty      DECIMAL(18,3),
    last_supplier_id       BINARY(16),
    last_purchase_item_id  BINARY(16),
    status                 VARCHAR(20)   NOT NULL DEFAULT 'active',
    version                INT           NOT NULL DEFAULT 1,
    created_at             DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at             DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    deleted_at             DATETIME(6),
    live_sku               VARCHAR(64)   AS (IF(deleted_at IS NULL, sku, NULL)) VIRTUAL,
    default_key            TINYINT       AS (IF(is_default, 1, NULL)) VIRTUAL,
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_variants_sku (tenant_id, live_sku),
    UNIQUE KEY uq_variants_default (tenant_id, product_id, default_key),
    KEY idx_variants_product (tenant_id, product_id),
    KEY idx_variants_search_prefix (tenant_id, search_text(64)),
    FULLTEXT KEY ft_variants_search (search_text) WITH PARSER ngram,
    CONSTRAINT fk_variants_tenant  FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_variants_product FOREIGN KEY (tenant_id, product_id) REFERENCES products (tenant_id, id),
    CONSTRAINT ck_variants_sku     CHECK (REGEXP_LIKE(sku, '^[A-Za-z0-9._-]{1,64}$')),
    CONSTRAINT ck_variants_prices  CHECK (purchase_price >= 0 AND selling_price >= 0
                                          AND (wholesale_price IS NULL OR wholesale_price >= 0)
                                          AND (min_selling_price IS NULL OR min_selling_price >= 0)
                                          AND (reorder_level IS NULL OR reorder_level >= 0)),
    CONSTRAINT ck_variants_status  CHECK (status IN ('active','archived'))
);

CREATE TABLE product_barcodes (
    tenant_id   BINARY(16)  NOT NULL,
    id          BINARY(16)  NOT NULL,
    variant_id  BINARY(16)  NOT NULL,
    barcode     VARCHAR(64) NOT NULL,
    symbology   VARCHAR(10),
    unit_id     BINARY(16),
    is_primary  BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_barcodes (tenant_id, barcode),
    KEY idx_barcodes_variant (tenant_id, variant_id),
    CONSTRAINT fk_barcodes_variant FOREIGN KEY (tenant_id, variant_id) REFERENCES product_variants (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_barcodes_unit    FOREIGN KEY (tenant_id, unit_id)    REFERENCES units (tenant_id, id),
    CONSTRAINT ck_barcodes_symbology CHECK (symbology IS NULL OR symbology IN ('ean13','ean8','upca','code128','qr','other'))
);

CREATE TABLE product_price_history (
    tenant_id    BINARY(16)    NOT NULL,
    id           BINARY(16)    NOT NULL,
    variant_id   BINARY(16)    NOT NULL,
    price_field  VARCHAR(20)   NOT NULL,
    old_value    DECIMAL(18,4),
    new_value    DECIMAL(18,4),
    changed_by   BINARY(16),
    changed_at   DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_price_history_variant (tenant_id, variant_id, changed_at),
    CONSTRAINT fk_price_history_variant FOREIGN KEY (tenant_id, variant_id) REFERENCES product_variants (tenant_id, id),
    CONSTRAINT ck_price_history_field CHECK (price_field IN ('selling_price','purchase_price','wholesale_price','min_selling_price'))
);

ALTER TABLE tenant_settings ADD CONSTRAINT fk_settings_default_tax
    FOREIGN KEY (tenant_id, default_tax_rate_id) REFERENCES tax_rates (tenant_id, id);

-- Defaults for businesses provisioned before this migration (DB design §13.2;
-- new businesses get them from provisioning). The new tables' guard triggers
-- are created below; tenant_settings already has one, hence platform mode.
SET @app_platform = 1;
INSERT INTO tax_rates (tenant_id, id, name, rate_pct, is_default)
SELECT t.id, UUID_TO_BIN(UUID(), 1), r.name, r.rate_pct, r.is_default
FROM tenants t
CROSS JOIN (SELECT 'No VAT' AS name, 0 AS rate_pct, TRUE AS is_default
            UNION ALL SELECT 'VAT 5%', 5, FALSE UNION ALL SELECT 'VAT 7.5%', 7.5, FALSE
            UNION ALL SELECT 'VAT 10%', 10, FALSE UNION ALL SELECT 'VAT 15%', 15, FALSE) r
WHERE t.slug <> 'platform';

INSERT INTO units (tenant_id, id, name, short_name, allow_fraction)
SELECT t.id, UUID_TO_BIN(UUID(), 1), u.name, u.short_name, u.allow_fraction
FROM tenants t
CROSS JOIN (SELECT 'Piece' AS name, 'pcs' AS short_name, FALSE AS allow_fraction
            UNION ALL SELECT 'Kilogram', 'kg', TRUE UNION ALL SELECT 'Gram', 'g', TRUE
            UNION ALL SELECT 'Litre', 'L', TRUE UNION ALL SELECT 'Millilitre', 'ml', TRUE
            UNION ALL SELECT 'Box', 'box', FALSE UNION ALL SELECT 'Dozen', 'dz', FALSE
            UNION ALL SELECT 'Pack', 'pack', FALSE) u
WHERE t.slug <> 'platform';

UPDATE tenant_settings s
JOIN tax_rates r ON r.tenant_id = s.tenant_id AND r.is_default
SET s.default_tax_rate_id = r.id
WHERE s.default_tax_rate_id IS NULL;

-- Codes are `P000042` (Requirement Analysis §7.5); 0004 provisioned them with 5 digits.
UPDATE document_sequences SET padding = 6
WHERE doc_type IN ('product','customer','supplier') AND next_value = 1;
SET @app_platform = NULL;

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- tax_rates
CREATE TRIGGER trg_tax_rates_tg_ins BEFORE INSERT ON tax_rates FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tax_rates_tg_upd BEFORE UPDATE ON tax_rates FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tax_rates_tg_del BEFORE DELETE ON tax_rates FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- units
CREATE TRIGGER trg_units_tg_ins BEFORE INSERT ON units FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_units_tg_upd BEFORE UPDATE ON units FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_units_tg_del BEFORE DELETE ON units FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- categories
CREATE TRIGGER trg_categories_tg_ins BEFORE INSERT ON categories FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_categories_tg_upd BEFORE UPDATE ON categories FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_categories_tg_del BEFORE DELETE ON categories FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- brands
CREATE TRIGGER trg_brands_tg_ins BEFORE INSERT ON brands FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_brands_tg_upd BEFORE UPDATE ON brands FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_brands_tg_del BEFORE DELETE ON brands FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- products
CREATE TRIGGER trg_products_tg_ins BEFORE INSERT ON products FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_products_tg_upd BEFORE UPDATE ON products FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_products_tg_del BEFORE DELETE ON products FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- product_variants
CREATE TRIGGER trg_product_variants_tg_ins BEFORE INSERT ON product_variants FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_variants_tg_upd BEFORE UPDATE ON product_variants FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_variants_tg_del BEFORE DELETE ON product_variants FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- product_barcodes
CREATE TRIGGER trg_product_barcodes_tg_ins BEFORE INSERT ON product_barcodes FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_barcodes_tg_upd BEFORE UPDATE ON product_barcodes FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_barcodes_tg_del BEFORE DELETE ON product_barcodes FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- product_price_history
CREATE TRIGGER trg_product_price_history_tg_ins BEFORE INSERT ON product_price_history FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_price_history_tg_upd BEFORE UPDATE ON product_price_history FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_product_price_history_tg_del BEFORE DELETE ON product_price_history FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
