-- 0007 audit log, idempotency keys, transactional outbox (DB design §9.6)
-- audit_logs is partitioned by month and has no foreign keys (MySQL restriction).
-- The worker keeps three future partitions (crates/worker/src/partitions.rs).

CREATE TABLE audit_logs (
    id            BINARY(16)   NOT NULL,
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    tenant_id     BINARY(16),
    actor_type    VARCHAR(20)  NOT NULL,
    actor_id      BINARY(16),
    action        VARCHAR(60)  NOT NULL,
    entity_type   VARCHAR(40),
    entity_id     BINARY(16),
    branch_id     BINARY(16),
    before_data   JSON,
    after_data    JSON,
    reason        VARCHAR(500),
    ip_address    VARBINARY(16),
    user_agent    VARCHAR(500),
    request_id    VARCHAR(64),
    PRIMARY KEY (id, created_at),
    KEY idx_audit_tenant_time (tenant_id, created_at),
    KEY idx_audit_entity (tenant_id, entity_type, entity_id),
    KEY idx_audit_actor (tenant_id, actor_id, created_at),
    CONSTRAINT ck_audit_actor CHECK (actor_type IN ('user','platform_staff','system','api_key'))
)
PARTITION BY RANGE COLUMNS (created_at) (
    PARTITION p2026_10 VALUES LESS THAN ('2026-11-01'),
    PARTITION p2026_11 VALUES LESS THAN ('2026-12-01'),
    PARTITION p2026_12 VALUES LESS THAN ('2027-01-01'),
    PARTITION p2027_01 VALUES LESS THAN ('2027-02-01'),
    PARTITION pmax     VALUES LESS THAN (MAXVALUE)
);

CREATE TABLE idempotency_keys (
    tenant_id        BINARY(16)   NOT NULL,
    idem_key         BINARY(16)   NOT NULL,
    user_id          BINARY(16)   NOT NULL,
    endpoint         VARCHAR(100) NOT NULL,
    request_hash     BINARY(32)   NOT NULL,
    status           VARCHAR(20)  NOT NULL DEFAULT 'in_progress',
    response_status  SMALLINT,
    response_body    JSON,
    resource_type    VARCHAR(40),
    resource_id      BINARY(16),
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at       DATETIME(6)  NOT NULL,
    PRIMARY KEY (tenant_id, idem_key),
    KEY idx_idempotency_expiry (expires_at),
    CONSTRAINT fk_idempotency_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_idempotency_status CHECK (status IN ('in_progress','completed'))
);

CREATE TABLE outbox_events (
    tenant_id       BINARY(16)  NOT NULL,
    id              BINARY(16)  NOT NULL,
    event_type      VARCHAR(60) NOT NULL,
    aggregate_type  VARCHAR(40) NOT NULL,
    aggregate_id    BINARY(16)  NOT NULL,
    payload         JSON        NOT NULL,
    created_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    published_at    DATETIME(6),
    attempts        INT         NOT NULL DEFAULT 0,
    last_error      VARCHAR(1000),
    PRIMARY KEY (tenant_id, id),
    KEY idx_outbox_unpublished (published_at, created_at),
    CONSTRAINT fk_outbox_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- audit_logs
CREATE TRIGGER trg_audit_logs_ao_upd BEFORE UPDATE ON audit_logs FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_audit_logs_ao_del BEFORE DELETE ON audit_logs FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- idempotency_keys
CREATE TRIGGER trg_idempotency_keys_tg_ins BEFORE INSERT ON idempotency_keys FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_idempotency_keys_tg_upd BEFORE UPDATE ON idempotency_keys FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_idempotency_keys_tg_del BEFORE DELETE ON idempotency_keys FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- outbox_events
CREATE TRIGGER trg_outbox_events_tg_ins BEFORE INSERT ON outbox_events FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_outbox_events_tg_upd BEFORE UPDATE ON outbox_events FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_outbox_events_tg_del BEFORE DELETE ON outbox_events FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
