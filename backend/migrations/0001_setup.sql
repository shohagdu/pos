-- 0001 setup
-- The database itself, users and roles are created per environment by infra
-- (infra/mysql/init/ for local development; Terraform/managed service in cloud).
-- This migration only enforces the database-level defaults (DB design §2, §8).
ALTER DATABASE CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
