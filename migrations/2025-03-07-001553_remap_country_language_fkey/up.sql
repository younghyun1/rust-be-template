-- 1. Add a new column to store the integer language_code.
ALTER TABLE public.iso_country
ADD COLUMN country_primary_language_code int4;

-- 2. Populate the new column by joining iso_country’s existing country_primary_language (bpchar) with iso_language’s language_alpha2.
UPDATE public.iso_country c
SET
    country_primary_language_code = l.language_code
FROM
    public.iso_language l
WHERE
    c.country_primary_language = l.language_alpha2;

-- 3. Verify that all rows got mapped as expected.
-- SELECT count(*) FROM public.iso_country WHERE country_primary_language_code IS NULL;
-- 4. Drop the old foreign key constraint.
ALTER TABLE public.iso_country
DROP CONSTRAINT iso_country_country_primary_language_fkey;

-- 5. Remove the old bpchar column.
ALTER TABLE public.iso_country
DROP COLUMN country_primary_language;

-- 6. Rename the new column to country_primary_language and enforce NOT NULL.
ALTER TABLE public.iso_country
RENAME COLUMN country_primary_language_code TO country_primary_language;

ALTER TABLE public.iso_country
ALTER COLUMN country_primary_language
SET
    NOT NULL;

-- 7. Recreate the foreign key constraint to reference iso_language(language_code).
ALTER TABLE public.iso_country ADD CONSTRAINT iso_country_country_primary_language_fkey FOREIGN KEY (country_primary_language) REFERENCES public.iso_language (language_code);

-- 8. Drop the outdated index on country_primary_language and recreate it on the new int4 column.
DROP INDEX IF EXISTS idx_country_language;

CREATE INDEX idx_country_language ON public.iso_country USING btree (country_primary_language);