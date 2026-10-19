-- 0003 identity: global users, sessions, OTP, auth events (DB design §9.2)

CREATE TABLE users (
    id                  BINARY(16)   NOT NULL,
    full_name           VARCHAR(150) NOT NULL,
    phone               VARCHAR(16),
    email               VARCHAR(254),
    password_hash       VARCHAR(255) NOT NULL,
    status              VARCHAR(20)  NOT NULL DEFAULT 'active',
    phone_verified_at   DATETIME(6),
    email_verified_at   DATETIME(6),
    locale              VARCHAR(10)  NOT NULL DEFAULT 'en',
    mfa_totp_secret_enc VARBINARY(255),
    mfa_enabled         BOOLEAN      NOT NULL DEFAULT FALSE,
    failed_login_count  INT          NOT NULL DEFAULT 0,
    locked_until        DATETIME(6),
    last_login_at       DATETIME(6),
    created_at          DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at          DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_users_phone (phone),
    UNIQUE KEY uq_users_email (email),
    CONSTRAINT ck_users_phone   CHECK (phone IS NULL OR REGEXP_LIKE(phone, '^\\+?[0-9]{8,15}$')),
    CONSTRAINT ck_users_contact CHECK (phone IS NOT NULL OR email IS NOT NULL),
    CONSTRAINT ck_users_status  CHECK (status IN ('active','disabled'))
);

CREATE TABLE user_sessions (
    id                  BINARY(16)   NOT NULL,
    user_id             BINARY(16)   NOT NULL,
    tenant_id           BINARY(16),
    family_id           BINARY(16)   NOT NULL,
    refresh_token_hash  BINARY(32)   NOT NULL,
    replaced_by_id      BINARY(16),
    ip_address          VARBINARY(16),
    user_agent          VARCHAR(500),
    expires_at          DATETIME(6)  NOT NULL,
    last_used_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    revoked_at          DATETIME(6),
    revoked_reason      VARCHAR(40),
    created_at          DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_user_sessions_token (refresh_token_hash),
    KEY idx_user_sessions_user (user_id, revoked_at),
    KEY idx_user_sessions_family (family_id),
    CONSTRAINT fk_user_sessions_user   FOREIGN KEY (user_id)   REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_user_sessions_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id)
);

CREATE TABLE otp_codes (
    id           BINARY(16)   NOT NULL,
    user_id      BINARY(16),
    destination  VARCHAR(254) NOT NULL,
    purpose      VARCHAR(20)  NOT NULL,
    code_hash    BINARY(32)   NOT NULL,
    attempts     SMALLINT     NOT NULL DEFAULT 0,
    expires_at   DATETIME(6)  NOT NULL,
    consumed_at  DATETIME(6),
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY idx_otp_destination (destination, purpose, created_at),
    CONSTRAINT fk_otp_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT ck_otp_purpose CHECK (purpose IN ('verify_phone','verify_email','reset_password','login_mfa'))
);

CREATE TABLE auth_events (
    id          BINARY(16)   NOT NULL,
    user_id     BINARY(16),
    tenant_id   BINARY(16),
    event       VARCHAR(40)  NOT NULL,
    identifier  VARCHAR(254),
    ip_address  VARBINARY(16),
    user_agent  VARCHAR(500),
    metadata    JSON         NOT NULL DEFAULT (JSON_OBJECT()),
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY idx_auth_events_user (user_id, created_at),
    KEY idx_auth_events_tenant (tenant_id, created_at)
);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- auth_events
CREATE TRIGGER trg_auth_events_ao_upd BEFORE UPDATE ON auth_events FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_auth_events_ao_del BEFORE DELETE ON auth_events FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;

-- <<< GENERATED GUARDS
