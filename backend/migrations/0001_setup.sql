-- 0001 setup
-- The database itself, users and roles are created per environment by infra
-- (infra/mysql/init/ for local development; Terraform/managed service in cloud).
-- This migration only enforces the database-level defaults (DB design §2, §8).
-- utf8mb4_general_ci exists on both MySQL 8 and MariaDB (cPanel hosts have
-- no 0900 collations).
ALTER DATABASE CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;
