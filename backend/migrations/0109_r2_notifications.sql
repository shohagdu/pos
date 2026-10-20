-- 0109 notifications (DB design §9.17, NOT-R01, NOT-R02).
--
-- Each recipient gets an in-app notification row; the channels in their
-- preferences (email, SMS) get a delivery row that the worker sends with
-- retries (exponential backoff, at most 5 tries over about an hour).
-- `dedupe_key` lets the scheduled checks run again safely: one notification
-- per user and key (for example one low-stock digest per day).

CREATE TABLE notifications (
    tenant_id   BINARY(16)   NOT NULL,
    id          BINARY(16)   NOT NULL,
    user_id     BINARY(16)   NOT NULL,
    notif_type  VARCHAR(50)  NOT NULL,
    title       VARCHAR(200) NOT NULL,
    body        VARCHAR(1000),
    link        VARCHAR(300),
    data        JSON         NOT NULL DEFAULT (JSON_OBJECT()),
    dedupe_key  VARCHAR(150),
    read_at     DATETIME(6),
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_notifications_dedupe (tenant_id, user_id, dedupe_key),
    KEY idx_notifications_user (tenant_id, user_id, read_at, created_at),
    KEY idx_notifications_dedupe (tenant_id, dedupe_key),
    CONSTRAINT fk_notifications_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_notifications_user   FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE notification_preferences (
    tenant_id   BINARY(16)  NOT NULL,
    user_id     BINARY(16)  NOT NULL,
    notif_type  VARCHAR(50) NOT NULL,
    -- Any of "in_app", "email", "sms"; an empty list turns the type off.
    channels    JSON        NOT NULL DEFAULT (JSON_ARRAY('in_app')),
    updated_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, user_id, notif_type),
    CONSTRAINT fk_notif_prefs_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_notif_prefs_user   FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE notification_deliveries (
    tenant_id        BINARY(16)   NOT NULL,
    id               BINARY(16)   NOT NULL,
    notification_id  BINARY(16)   NOT NULL,
    channel          VARCHAR(10)  NOT NULL,
    destination      VARCHAR(254) NOT NULL,
    status           VARCHAR(10)  NOT NULL DEFAULT 'queued',
    attempts         SMALLINT     NOT NULL DEFAULT 0,
    provider_ref     VARCHAR(100),
    last_error       VARCHAR(1000),
    next_attempt_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    sent_at          DATETIME(6),
    PRIMARY KEY (tenant_id, id),
    KEY idx_notif_deliveries_due (status, next_attempt_at),
    KEY idx_notif_deliveries_notification (tenant_id, notification_id),
    CONSTRAINT fk_notif_deliveries_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_notif_deliveries_notification FOREIGN KEY (tenant_id, notification_id) REFERENCES notifications (tenant_id, id),
    CONSTRAINT ck_notif_deliveries_channel CHECK (channel IN ('email', 'sms')),
    CONSTRAINT ck_notif_deliveries_status  CHECK (status IN ('queued', 'sent', 'failed', 'gave_up'))
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- notifications
CREATE TRIGGER trg_notifications_tg_ins BEFORE INSERT ON notifications FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_notifications_tg_upd BEFORE UPDATE ON notifications FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_notifications_tg_del BEFORE DELETE ON notifications FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- notification_preferences
CREATE TRIGGER trg_notification_preferences_tg_ins BEFORE INSERT ON notification_preferences FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_notification_preferences_tg_upd BEFORE UPDATE ON notification_preferences FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_notification_preferences_tg_del BEFORE DELETE ON notification_preferences FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- notification_deliveries
CREATE TRIGGER trg_notification_deliveries_tg_ins BEFORE INSERT ON notification_deliveries FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_notification_deliveries_tg_upd BEFORE UPDATE ON notification_deliveries FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_notification_deliveries_tg_del BEFORE DELETE ON notification_deliveries FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
