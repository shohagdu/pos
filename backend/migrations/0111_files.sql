-- 0111 files (DB design §9.9): uploaded files are kept in object storage
-- under `tenants/{tenant_id}/…`; this table records what each one is. The
-- first user is product images (`products.image_file_id`, ≤ 2 MB jpg/png/webp).
-- `storage.mb` counts the sizes recorded here.

CREATE TABLE files (
    tenant_id        BINARY(16)   NOT NULL,
    id               BINARY(16)   NOT NULL,
    storage_key      VARCHAR(300) NOT NULL,
    file_name        VARCHAR(255) NOT NULL,
    content_type     VARCHAR(100) NOT NULL,
    size_bytes       BIGINT       NOT NULL,
    checksum_sha256  BINARY(32),
    purpose          VARCHAR(20)  NOT NULL,
    entity_type      VARCHAR(40),
    entity_id        BINARY(16),
    uploaded_by      BINARY(16),
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at       DATETIME(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_files_storage_key (storage_key),
    KEY idx_files_entity (tenant_id, entity_type, entity_id),
    CONSTRAINT fk_files_tenant  FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_files_size    CHECK (size_bytes > 0),
    CONSTRAINT ck_files_purpose CHECK (purpose IN ('product_image','logo','attachment','import','export','payment_proof'))
);

ALTER TABLE products
    ADD CONSTRAINT fk_products_image FOREIGN KEY (tenant_id, image_file_id) REFERENCES files (tenant_id, id);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- files
CREATE TRIGGER trg_files_tg_ins BEFORE INSERT ON files FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_files_tg_upd BEFORE UPDATE ON files FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_files_tg_del BEFORE DELETE ON files FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
