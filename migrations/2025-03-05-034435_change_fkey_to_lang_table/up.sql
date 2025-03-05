-- Step 1: Drop the existing foreign key constraint on country_primary_language
ALTER TABLE public.iso_country
DROP CONSTRAINT iso_country_country_primary_language_fkey;

-- Step 2: Add the foreign key constraint using language_alpha2 instead of language_code
-- Ensure that country_primary_language is of the same type as language_alpha2, using bpchar(2)
ALTER TABLE public.iso_country
ALTER COLUMN country_primary_language TYPE bpchar(2)
USING country_primary_language::bpchar;

ALTER TABLE public.iso_country ADD CONSTRAINT iso_country_country_primary_language_fkey FOREIGN KEY (country_primary_language) REFERENCES public.iso_language (language_alpha2);

-- Step 3: Create indexes on relevant columns in the 'iso_country' table
CREATE UNIQUE INDEX IF NOT EXISTS idx_country_alpha2 ON public.iso_country (country_alpha2);

CREATE UNIQUE INDEX IF NOT EXISTS idx_country_alpha3 ON public.iso_country (country_alpha3);

CREATE INDEX IF NOT EXISTS idx_phone_prefix ON public.iso_country (phone_prefix);

CREATE INDEX IF NOT EXISTS idx_country_eng_name ON public.iso_country (country_eng_name);

CREATE INDEX IF NOT EXISTS idx_country_language ON public.iso_country (country_primary_language);

CREATE INDEX IF NOT EXISTS idx_country_currency ON public.iso_country (country_currency);

-- Step 4: Create indexes on relevant columns in the 'iso_language' table
CREATE UNIQUE INDEX IF NOT EXISTS idx_language_alpha2 ON public.iso_language (language_alpha2);

CREATE UNIQUE INDEX IF NOT EXISTS idx_language_alpha3 ON public.iso_language (language_alpha3);

CREATE INDEX IF NOT EXISTS idx_language_eng_name ON public.iso_language (language_eng_name);
