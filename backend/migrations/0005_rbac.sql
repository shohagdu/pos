-- 0005 RBAC (DB design §9.4, Requirement Analysis §10.1)

CREATE TABLE permissions (
    code          VARCHAR(60)  NOT NULL,
    module        VARCHAR(30)  NOT NULL,
    description   VARCHAR(255) NOT NULL,
    is_sensitive  BOOLEAN      NOT NULL DEFAULT FALSE,
    PRIMARY KEY (code)
);

CREATE TABLE roles (
    tenant_id         BINARY(16)   NOT NULL,
    id                BINARY(16)   NOT NULL,
    code              VARCHAR(40)  NOT NULL,
    name              VARCHAR(100) NOT NULL,
    description       VARCHAR(255),
    is_system         BOOLEAN      NOT NULL DEFAULT FALSE,
    max_discount_pct  DECIMAL(7,4),
    version           INT          NOT NULL DEFAULT 1,
    created_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_roles_code (tenant_id, code),
    CONSTRAINT fk_roles_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id),
    CONSTRAINT ck_roles_discount CHECK (max_discount_pct IS NULL OR max_discount_pct BETWEEN 0 AND 100)
);

CREATE TABLE role_permissions (
    tenant_id        BINARY(16)  NOT NULL,
    role_id          BINARY(16)  NOT NULL,
    permission_code  VARCHAR(60) NOT NULL,
    PRIMARY KEY (tenant_id, role_id, permission_code),
    CONSTRAINT fk_role_perms_role FOREIGN KEY (tenant_id, role_id) REFERENCES roles (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_role_perms_perm FOREIGN KEY (permission_code) REFERENCES permissions (code)
);

CREATE TABLE membership_roles (
    tenant_id      BINARY(16)  NOT NULL,
    id             BINARY(16)  NOT NULL,
    membership_id  BINARY(16)  NOT NULL,
    role_id        BINARY(16)  NOT NULL,
    branch_id      BINARY(16),
    branch_key     BINARY(16)  AS (COALESCE(branch_id, 0x00000000000000000000000000000000)) VIRTUAL,
    created_at     DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    created_by     BINARY(16),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_membership_roles (tenant_id, membership_id, role_id, branch_key),
    CONSTRAINT fk_mr_membership FOREIGN KEY (tenant_id, membership_id) REFERENCES tenant_memberships (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT fk_mr_role       FOREIGN KEY (tenant_id, role_id)       REFERENCES roles (tenant_id, id),
    CONSTRAINT fk_mr_branch     FOREIGN KEY (tenant_id, branch_id)     REFERENCES branches (tenant_id, id)
);

INSERT INTO permissions (code, module, description, is_sensitive) VALUES
 ('dashboard.view',              'dashboard',  'View dashboard',                              FALSE),
 ('dashboard.view_financials',   'dashboard',  'View financial figures on dashboard',         TRUE),
 ('pos.sell',                    'pos',        'Use the POS and complete sales',              FALSE),
 ('pos.price_override',          'pos',        'Change the unit price in the POS',            TRUE),
 ('pos.discount_over_limit',     'pos',        'Give discounts above the role limit',         TRUE),
 ('pos.override_stock',          'pos',        'Sell when stock is insufficient',             TRUE),
 ('pos.below_min_price',         'pos',        'Sell below minimum selling price',            TRUE),
 ('sales.view',                  'sales',      'View own sales',                              FALSE),
 ('sales.view_all_users',        'sales',      'View sales of all users',                     FALSE),
 ('sales.cancel',                'sales',      'Cancel (void) a completed sale',              TRUE),
 ('sales.return',                'sales',      'Process sale returns',                        TRUE),
 ('sales.return_override',       'sales',      'Return outside the return window',            TRUE),
 ('sales.override_credit_limit', 'sales',      'Exceed a customer credit limit',              TRUE),
 ('sales.backdate',              'sales',      'Record sales with a past date',               TRUE),
 ('products.view',               'products',   'View products',                               FALSE),
 ('products.create',             'products',   'Create products',                             FALSE),
 ('products.update',             'products',   'Update products and prices',                  FALSE),
 ('products.archive',            'products',   'Archive products',                            FALSE),
 ('products.import',             'products',   'Import products from files',                  FALSE),
 ('products.view_cost',          'products',   'View purchase cost and margins',              TRUE),
 ('inventory.view',              'inventory',  'View stock',                                  FALSE),
 ('inventory.adjust',            'inventory',  'Post stock adjustments',                      TRUE),
 ('inventory.opening_stock',     'inventory',  'Enter opening stock',                         TRUE),
 ('inventory.transfer',          'inventory',  'Create and dispatch stock transfers',         FALSE),
 ('inventory.transfer_receive',  'inventory',  'Receive stock transfers',                     FALSE),
 ('inventory.count',             'inventory',  'Run stock counts',                            FALSE),
 ('purchases.view',              'purchases',  'View purchases',                              FALSE),
 ('purchases.create',            'purchases',  'Create purchase drafts and orders',           FALSE),
 ('purchases.post',              'purchases',  'Post purchase bills',                         FALSE),
 ('purchases.approve',           'purchases',  'Approve purchase orders',                     TRUE),
 ('purchases.cancel',            'purchases',  'Cancel purchases',                            TRUE),
 ('purchases.return',            'purchases',  'Process purchase returns',                    TRUE),
 ('customers.view',              'customers',  'View customers',                              FALSE),
 ('customers.manage',            'customers',  'Create and update customers',                 FALSE),
 ('customers.collect_payment',   'customers',  'Collect customer payments',                   FALSE),
 ('suppliers.view',              'suppliers',  'View suppliers',                              FALSE),
 ('suppliers.manage',            'suppliers',  'Create and update suppliers',                 FALSE),
 ('suppliers.pay',               'suppliers',  'Pay suppliers',                               TRUE),
 ('accounts.view',               'money',      'View cash/bank accounts and books',           FALSE),
 ('accounts.manage',             'money',      'Manage cash/bank accounts',                   TRUE),
 ('payments.void',               'money',      'Void posted payments',                        TRUE),
 ('transfers.create',            'money',      'Transfer money between accounts',             TRUE),
 ('expenses.view',               'money',      'View expenses',                               FALSE),
 ('expenses.create',             'money',      'Record expenses',                             FALSE),
 ('expenses.approve',            'money',      'Approve expenses',                            TRUE),
 ('income.create',               'money',      'Record other income',                         FALSE),
 ('accounting.view',             'accounting', 'View ledgers and statements',                 FALSE),
 ('accounting.journal_create',   'accounting', 'Create manual journal entries',               TRUE),
 ('accounting.period_lock',      'accounting', 'Lock and unlock fiscal periods',              TRUE),
 ('reports.sales',               'reports',    'View sales reports',                          FALSE),
 ('reports.purchases',           'reports',    'View purchase reports',                       FALSE),
 ('reports.inventory',           'reports',    'View inventory reports',                      FALSE),
 ('reports.parties',             'reports',    'View customer and supplier reports',          FALSE),
 ('reports.financial',           'reports',    'View financial reports',                      TRUE),
 ('reports.export',              'reports',    'Export reports',                              TRUE),
 ('users.view',                  'users',      'View users',                                  FALSE),
 ('users.invite',                'users',      'Invite users',                                TRUE),
 ('users.manage_roles',          'users',      'Change roles and permissions',                TRUE),
 ('users.revoke_sessions',       'users',      'Revoke user sessions',                        TRUE),
 ('settings.view',               'settings',   'View business settings',                      FALSE),
 ('settings.manage',             'settings',   'Change business settings',                    TRUE),
 ('branches.manage',             'settings',   'Manage branches',                             TRUE),
 ('subscription.manage',         'settings',   'Manage subscription and billing',             TRUE),
 ('audit.view',                  'audit',      'View audit log',                              TRUE);

-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand

-- roles
CREATE TRIGGER trg_roles_tg_ins BEFORE INSERT ON roles FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_roles_tg_upd BEFORE UPDATE ON roles FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_roles_tg_del BEFORE DELETE ON roles FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- role_permissions
CREATE TRIGGER trg_role_permissions_tg_ins BEFORE INSERT ON role_permissions FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_role_permissions_tg_upd BEFORE UPDATE ON role_permissions FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_role_permissions_tg_del BEFORE DELETE ON role_permissions FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- membership_roles
CREATE TRIGGER trg_membership_roles_tg_ins BEFORE INSERT ON membership_roles FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_membership_roles_tg_upd BEFORE UPDATE ON membership_roles FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;
CREATE TRIGGER trg_membership_roles_tg_del BEFORE DELETE ON membership_roles FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;

-- <<< GENERATED GUARDS
