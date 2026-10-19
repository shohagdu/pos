-- 0104 r2_pos (DB design §9.12 Release 2): cash registers and shifts.
--
-- A register is a till in a branch with its own cash account. A cashier
-- opens a shift with the float, sells, and closes it by counting the drawer:
-- expected cash = opening float + cash taken − cash paid out on the
-- register's account during the shift. Sales and payments made while a shift
-- is open carry its id (`register_shift_id`, added in 0012/0014).

CREATE TABLE cash_registers (
    tenant_id        BINARY(16)   NOT NULL,
    id               BINARY(16)   NOT NULL,
    branch_id        BINARY(16)   NOT NULL,
    name             VARCHAR(50)  NOT NULL,
    cash_account_id  BINARY(16)   NOT NULL,
    status           VARCHAR(20)  NOT NULL DEFAULT 'active',
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_by       BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_cash_registers_name (tenant_id, branch_id, name),
    CONSTRAINT fk_registers_branch  FOREIGN KEY (tenant_id, branch_id)       REFERENCES branches (tenant_id, id),
    CONSTRAINT fk_registers_account FOREIGN KEY (tenant_id, cash_account_id) REFERENCES payment_accounts (tenant_id, id),
    CONSTRAINT ck_registers_status  CHECK (status IN ('active','inactive'))
);

CREATE TABLE register_shifts (
    tenant_id        BINARY(16)    NOT NULL,
    id               BINARY(16)    NOT NULL,
    register_id      BINARY(16)    NOT NULL,
    branch_id        BINARY(16)    NOT NULL,
    shift_no         INT           NOT NULL,
    opened_by        BINARY(16)    NOT NULL,
    opened_at        DATETIME(6)   NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    opening_cash     DECIMAL(18,2) NOT NULL,
    closed_by        BINARY(16),
    closed_at        DATETIME(6),
    expected_cash    DECIMAL(18,2),
    counted_cash     DECIMAL(18,2),
    difference       DECIMAL(18,2) AS (counted_cash - expected_cash) STORED,
    status           VARCHAR(10)   NOT NULL DEFAULT 'open',
    open_key         TINYINT       AS (IF(status = 'open', 1, NULL)) VIRTUAL,
    acknowledged_by  BINARY(16),
    note             VARCHAR(500),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_register_open_shift (tenant_id, register_id, open_key),   -- one open shift per register
    UNIQUE KEY uq_register_shift_no (tenant_id, register_id, shift_no),
    KEY idx_shifts_user (tenant_id, opened_by, status),
    CONSTRAINT fk_shifts_register FOREIGN KEY (tenant_id, register_id) REFERENCES cash_registers (tenant_id, id),
    CONSTRAINT fk_shifts_branch   FOREIGN KEY (tenant_id, branch_id)   REFERENCES branches (tenant_id, id),
    CONSTRAINT ck_shifts_cash     CHECK (opening_cash >= 0 AND (counted_cash IS NULL OR counted_cash >= 0)),
    CONSTRAINT ck_shifts_status   CHECK (status IN ('open','closed'))
);

ALTER TABLE payments ADD KEY idx_payments_shift (tenant_id, register_shift_id);
ALTER TABLE sales ADD KEY idx_sales_shift (tenant_id, register_shift_id);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- cash_registers
CREATE TRIGGER trg_cash_registers_tg_ins BEFORE INSERT ON cash_registers FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_cash_registers_tg_upd BEFORE UPDATE ON cash_registers FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_cash_registers_tg_del BEFORE DELETE ON cash_registers FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- register_shifts
CREATE TRIGGER trg_register_shifts_tg_ins BEFORE INSERT ON register_shifts FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_register_shifts_tg_upd BEFORE UPDATE ON register_shifts FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_register_shifts_tg_del BEFORE DELETE ON register_shifts FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
