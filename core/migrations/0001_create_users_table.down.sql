-- 0001_create_users_table.down.sql
-- Rollback initial auth migration.

DROP TABLE IF EXISTS users;
