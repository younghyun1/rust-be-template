
DROP INDEX IF EXISTS idx_country_eng_name;
DROP INDEX IF EXISTS idx_country_language;
DROP INDEX IF EXISTS idx_country_currency;
DROP INDEX IF EXISTS idx_language_eng_name;

ALTER TABLE public.iso_country
  DROP CONSTRAINT IF EXISTS iso_country_country_primary_language_fkey;

ALTER TABLE public.iso_country
  ALTER COLUMN country_primary_language TYPE int4
  USING country_primary_language::int4;

ALTER TABLE public.iso_country
  ADD CONSTRAINT iso_country_country_primary_language_fkey
    FOREIGN KEY (country_primary_language) REFERENCES public.iso_language(language_code);
