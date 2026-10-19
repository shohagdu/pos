-- One-time setup for an existing local MySQL 8 (no Docker). Run as an admin:
--   sudo mysql < infra/mysql/local-setup.sql      (Ubuntu, root via auth_socket)
--   mysql -uroot -p < infra/mysql/local-setup.sql
-- Creates the same logins as the Compose init script (init/01-users.sql).
-- Development only. Safe to run again. The passwords differ from the Compose
-- ones because Ubuntu's MySQL enables validate_password (MEDIUM policy:
-- upper, lower, digit and symbol); set DATABASE_URL in backend/.env to match.

CREATE DATABASE IF NOT EXISTS pos CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE DATABASE IF NOT EXISTS pos_test CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;

-- Guard triggers are created by the migration user while binary logging is on.
SET PERSIST log_bin_trust_function_creators = 1;

CREATE ROLE IF NOT EXISTS pos_migrator, pos_app, pos_readonly, pos_maintenance;
GRANT ALL PRIVILEGES ON pos.* TO pos_migrator;
GRANT SELECT, INSERT, UPDATE, DELETE, EXECUTE ON pos.* TO pos_app;
GRANT SELECT ON pos.* TO pos_readonly;
GRANT SELECT, DELETE ON pos.* TO pos_maintenance;

-- Login used by `make api` (needs DDL for RUN_MIGRATIONS_ON_START).
-- Also @'localhost': an anonymous ''@'localhost' account would otherwise win
-- over user@'%' for local connections.
CREATE USER IF NOT EXISTS 'pos_app'@'%' IDENTIFIED BY 'Pos_app-Dev1';
CREATE USER IF NOT EXISTS 'pos_app'@'localhost' IDENTIFIED BY 'Pos_app-Dev1';
ALTER USER 'pos_app'@'%', 'pos_app'@'localhost' IDENTIFIED BY 'Pos_app-Dev1';
GRANT pos_migrator TO 'pos_app'@'%', 'pos_app'@'localhost';
SET DEFAULT ROLE pos_migrator TO 'pos_app'@'%', 'pos_app'@'localhost';

-- Integration tests: #[sqlx::test] creates and drops one database per test.
CREATE USER IF NOT EXISTS 'pos_test'@'%' IDENTIFIED BY 'Pos_test-Dev1';
CREATE USER IF NOT EXISTS 'pos_test'@'localhost' IDENTIFIED BY 'Pos_test-Dev1';
ALTER USER 'pos_test'@'%', 'pos_test'@'localhost' IDENTIFIED BY 'Pos_test-Dev1';
GRANT ALL PRIVILEGES ON *.* TO 'pos_test'@'%', 'pos_test'@'localhost';

SELECT user, host FROM mysql.user WHERE user IN ('pos_app', 'pos_test', '');
