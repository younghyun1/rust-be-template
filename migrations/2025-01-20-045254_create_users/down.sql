DROP INDEX IF EXISTS idx_users_updated_at;

DROP INDEX IF EXISTS idx_users_created_at;

DROP INDEX IF EXISTS idx_users_email;

DROP INDEX IF EXISTS idx_users_name;

DROP TABLE IF EXISTS users;

DROP EXTENSION IF EXISTS "uuid-ossp";
