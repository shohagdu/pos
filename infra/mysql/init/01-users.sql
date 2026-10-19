-- Local development only. Runs once when the MySQL volume is first created.
-- Production users/roles are provisioned by infrastructure code (DB design §4).

CREATE ROLE IF NOT EXISTS pos_migrator, pos_app, pos_readonly, pos_maintenance;
GRANT ALL PRIVILEGES ON pos.* TO pos_migrator;
GRANT SELECT, INSERT, UPDATE, DELETE, EXECUTE ON pos.* TO pos_app;
GRANT SELECT ON pos.* TO pos_readonly;
GRANT SELECT, DELETE ON pos.* TO pos_maintenance;

-- One login used by `cargo run` in development (needs DDL for RUN_MIGRATIONS_ON_START).
CREATE USER IF NOT EXISTS 'pos_app'@'%' IDENTIFIED BY 'pos_app_dev_only';
GRANT pos_migrator TO 'pos_app'@'%';
SET DEFAULT ROLE pos_migrator TO 'pos_app'@'%';

-- Integration tests: #[sqlx::test] creates and drops one database per test.
CREATE DATABASE IF NOT EXISTS pos_test CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE USER IF NOT EXISTS 'pos_test'@'%' IDENTIFIED BY 'pos_test_dev_only';
GRANT ALL PRIVILEGES ON *.* TO 'pos_test'@'%';
