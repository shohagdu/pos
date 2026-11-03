-- 0001 setup
-- The database itself, users and roles are created per environment by infra
-- (infra/mysql/init/ for local development; Terraform/managed service in cloud).
-- This migration only enforces the database-level defaults (DB design §2, §8).
-- utf8mb4's default collation is utf8mb4_0900_ai_ci on MySQL 8. MariaDB
-- (cPanel hosts) has no 0900 collations; the /*M! */ part runs only there.
ALTER DATABASE CHARACTER SET utf8mb4 /*M! COLLATE utf8mb4_unicode_520_ci */;
