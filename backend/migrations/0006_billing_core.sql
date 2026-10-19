-- 0006 subscriptions & billing core (DB design §9.5). Coupons and gateway
-- transactions arrive in Release 2 (0108).

CREATE TABLE subscriptions (
    tenant_id             BINARY(16)    NOT NULL,
    id                    BINARY(16)    NOT NULL,
    plan_id               BINARY(16)    NOT NULL,
    status                VARCHAR(20)   NOT NULL,
    billing_cycle         VARCHAR(20)   NOT NULL DEFAULT 'monthly',
    price_amount          DECIMAL(18,2) NOT NULL DEFAULT 0,
    currency_code         CHAR(3)       NOT NULL DEFAULT 'BDT',
    trial_ends_at         DATETIME(6),
    current_period_start  DATETIME(6),
    current_period_end    DATETIME(6),
    grace_ends_at         DATETIME(6),
    read_only_ends_at     DATETIME(6),
    cancel_at_period_end  BOOLEAN       NOT NULL DEFAULT FALSE,
    cancelled_at          DATETIME(6),
    pending_plan_id       BINARY(16),
    version               INT           NOT NULL DEFAULT 1,
    created_at            DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at            DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_subscriptions_tenant (tenant_id),
    KEY idx_subscriptions_due (status, current_period_end),
    CONSTRAINT fk_subscriptions_tenant  FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_subscriptions_plan    FOREIGN KEY (plan_id) REFERENCES plans (id),
    CONSTRAINT fk_subscriptions_pending FOREIGN KEY (pending_plan_id) REFERENCES plans (id),
    CONSTRAINT ck_subscriptions_status  CHECK (status IN ('trialing','active','grace','read_only',
                                                         'suspended','cancelled','terminated')),
    CONSTRAINT ck_subscriptions_cycle   CHECK (billing_cycle IN ('monthly','quarterly','annual','custom'))
);

CREATE TABLE subscription_events (
    tenant_id        BINARY(16)  NOT NULL,
    id               BINARY(16)  NOT NULL,
    subscription_id  BINARY(16)  NOT NULL,
    event_type       VARCHAR(40) NOT NULL,
    from_status      VARCHAR(20),
    to_status        VARCHAR(20),
    from_plan_id     BINARY(16),
    to_plan_id       BINARY(16),
    actor_type       VARCHAR(20) NOT NULL,
    actor_id         BINARY(16),
    reason           VARCHAR(500),
    metadata         JSON        NOT NULL DEFAULT (JSON_OBJECT()),
    created_at       DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    CONSTRAINT fk_sub_events_sub FOREIGN KEY (tenant_id, subscription_id) REFERENCES subscriptions (tenant_id, id),
    CONSTRAINT ck_sub_events_actor CHECK (actor_type IN ('system','tenant_user','platform_staff'))
);

CREATE TABLE tenant_feature_overrides (
    tenant_id     BINARY(16)   NOT NULL,
    id            BINARY(16)   NOT NULL,
    feature_code  VARCHAR(60)  NOT NULL,
    enabled       BOOLEAN,
    limit_value   BIGINT,
    expires_at    DATETIME(6),
    reason        VARCHAR(500) NOT NULL,
    created_by    BINARY(16)   NOT NULL,
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_feature_overrides (tenant_id, feature_code),
    CONSTRAINT fk_overrides_tenant  FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_overrides_feature FOREIGN KEY (feature_code) REFERENCES features (code),
    CONSTRAINT ck_overrides_limit   CHECK (limit_value IS NULL OR limit_value >= 0)
);

CREATE TABLE subscription_invoices (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    invoice_no       VARCHAR(30)   NOT NULL,
    plan_id          BINARY(16)    NOT NULL,
    billing_cycle    VARCHAR(20)   NOT NULL,
    period_start     DATETIME(6)   NOT NULL,
    period_end       DATETIME(6)   NOT NULL,
    subtotal         DECIMAL(18,2) NOT NULL,
    discount_amount  DECIMAL(18,2) NOT NULL DEFAULT 0,
    tax_amount       DECIMAL(18,2) NOT NULL DEFAULT 0,
    total            DECIMAL(18,2) NOT NULL,
    status           VARCHAR(10)   NOT NULL DEFAULT 'open',
    coupon_id        BINARY(16),
    paid_at          DATETIME(6),
    created_at       DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_sub_invoices_no (invoice_no),
    CONSTRAINT fk_sub_invoices_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT fk_sub_invoices_plan   FOREIGN KEY (plan_id) REFERENCES plans (id),
    CONSTRAINT ck_sub_invoices_amts   CHECK (subtotal >= 0 AND discount_amount >= 0 AND tax_amount >= 0 AND total >= 0),
    CONSTRAINT ck_sub_invoices_status CHECK (status IN ('open','paid','void'))
);

CREATE TABLE subscription_payment_claims (
    tenant_id          BINARY(16)    NOT NULL,
    id                 BINARY(16)    NOT NULL,
    invoice_id         BINARY(16)    NOT NULL,
    method             VARCHAR(20)   NOT NULL,
    amount             DECIMAL(18,2) NOT NULL,
    transaction_ref    VARCHAR(60)   NOT NULL,
    payer_account      VARCHAR(60),
    proof_file_id      BINARY(16),
    status             VARCHAR(20)   NOT NULL DEFAULT 'pending',
    reviewed_by        BINARY(16),
    reviewed_at        DATETIME(6),
    rejection_reason   VARCHAR(500),
    submitted_by       BINARY(16)    NOT NULL,
    created_at         DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_claims_txn (method, transaction_ref),
    CONSTRAINT fk_claims_invoice FOREIGN KEY (tenant_id, invoice_id) REFERENCES subscription_invoices (tenant_id, id),
    CONSTRAINT ck_claims_method  CHECK (method IN ('bkash','nagad','rocket','bank_transfer','cash','other')),
    CONSTRAINT ck_claims_amount  CHECK (amount > 0),
    CONSTRAINT ck_claims_status  CHECK (status IN ('pending','approved','rejected'))
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- subscriptions
CREATE TRIGGER trg_subscriptions_tg_ins BEFORE INSERT ON subscriptions FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscriptions_tg_upd BEFORE UPDATE ON subscriptions FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscriptions_tg_del BEFORE DELETE ON subscriptions FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- subscription_events
CREATE TRIGGER trg_subscription_events_tg_ins BEFORE INSERT ON subscription_events FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscription_events_ao_upd BEFORE UPDATE ON subscription_events FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_subscription_events_ao_del BEFORE DELETE ON subscription_events FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- tenant_feature_overrides
CREATE TRIGGER trg_tenant_feature_overrides_tg_ins BEFORE INSERT ON tenant_feature_overrides FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tenant_feature_overrides_tg_upd BEFORE UPDATE ON tenant_feature_overrides FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_tenant_feature_overrides_tg_del BEFORE DELETE ON tenant_feature_overrides FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- subscription_invoices
CREATE TRIGGER trg_subscription_invoices_tg_ins BEFORE INSERT ON subscription_invoices FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscription_invoices_tg_upd BEFORE UPDATE ON subscription_invoices FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscription_invoices_tg_del BEFORE DELETE ON subscription_invoices FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- subscription_payment_claims
CREATE TRIGGER trg_subscription_payment_claims_tg_ins BEFORE INSERT ON subscription_payment_claims FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscription_payment_claims_tg_upd BEFORE UPDATE ON subscription_payment_claims FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_subscription_payment_claims_tg_del BEFORE DELETE ON subscription_payment_claims FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
