-- 0112 chart of accounts management (ACC-R01): a business can add its own
-- ledger accounts and groups and edit them. System accounts keep their code,
-- type and place; only their name can change. `version` guards edits
-- (If-Match), like the other editable records.

ALTER TABLE ledger_accounts
    ADD COLUMN version INT NOT NULL DEFAULT 1 AFTER status;

INSERT INTO permissions (code, module, description, is_sensitive) VALUES
 ('accounting.coa_manage', 'accounting', 'Add and edit the chart of accounts', TRUE);

-- Existing businesses: the system roles that get it for new businesses.
SET @app_platform = 1;

INSERT IGNORE INTO role_permissions (tenant_id, role_id, permission_code)
SELECT r.tenant_id, r.id, 'accounting.coa_manage'
FROM roles r
WHERE r.is_system AND r.code IN ('owner', 'admin', 'accountant');

SET @app_platform = NULL;
