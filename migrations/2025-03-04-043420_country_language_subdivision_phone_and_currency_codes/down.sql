DROP TABLE IF EXISTS iso_country_language;
DROP TABLE IF EXISTS iso_country_subdivision;
DROP TABLE IF EXISTS iso_phone_prefix;
DROP TABLE IF EXISTS iso_country;
DROP TABLE IF EXISTS iso_currency;
DROP TABLE IF EXISTS iso_language;

ALTER TABLE public.users
    DROP CONSTRAINT IF EXISTS fk_user_country,
    DROP CONSTRAINT IF EXISTS fk_user_language,
    DROP CONSTRAINT IF EXISTS fk_user_subdivision,
    DROP COLUMN IF EXISTS user_country,
    DROP COLUMN IF EXISTS user_language,
    DROP COLUMN IF EXISTS user_subdivision;

DROP INDEX IF EXISTS idx_users_country;
DROP INDEX IF EXISTS idx_users_language;
DROP INDEX IF EXISTS idx_users_subdivision;