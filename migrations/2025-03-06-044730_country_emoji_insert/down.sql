DROP INDEX IF EXISTS idx_iso_country_is_country;

ALTER TABLE public.iso_country
DROP COLUMN is_country,
DROP COLUMN country_flag;

