-- Drop indices for optimization first.
DROP INDEX IF EXISTS idx_user_roles_role_id;

DROP INDEX IF EXISTS idx_user_roles_user_id;

DROP INDEX IF EXISTS idx_role_permissions_permission_id;

DROP INDEX IF EXISTS idx_role_permissions_role_id;

DROP INDEX IF EXISTS idx_permissions_permission_name;

DROP INDEX IF EXISTS idx_roles_role_name;

-- Drop tables in reverse order of creation to maintain referential integrity.
DROP TABLE IF EXISTS user_roles;

DROP TABLE IF EXISTS role_permissions;

DROP TABLE IF EXISTS permissions;

DROP TABLE IF EXISTS roles;
