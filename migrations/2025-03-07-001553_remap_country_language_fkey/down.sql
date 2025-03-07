-- 1. Drop the foreign key constraint on the integer column.
ALTER TABLE public.iso_country
DROP CONSTRAINT iso_country_country_primary_language_fkey;

-- 2. Add a temporary column to store the original bpchar(2) values.
ALTER TABLE public.iso_country
ADD COLUMN country_primary_language_char bpchar (2);

-- 3. Re-populate the temporary column by joining with iso_language.
UPDATE public.iso_country c
SET
    country_primary_language_char = l.language_alpha2
FROM
    public.iso_language l
WHERE
    c.country_primary_language = l.language_code;

-- 4. Drop the index on the integer column.
DROP INDEX IF EXISTS idx_country_language;

-- 5. Remove the integer column.
ALTER TABLE public.iso_country
DROP COLUMN country_primary_language;

-- 6. Rename the temporary column to country_primary_language.
ALTER TABLE public.iso_country
RENAME COLUMN country_primary_language_char TO country_primary_language;

-- 7. Re-impose the NOT NULL constraint.
ALTER TABLE public.iso_country
ALTER COLUMN country_primary_language
SET
    NOT NULL;

-- 8. Re-create the foreign key constraint to reference iso_language(language_alpha2).
ALTER TABLE public.iso_country ADD CONSTRAINT iso_country_country_primary_language_fkey FOREIGN KEY (country_primary_language) REFERENCES public.iso_language (language_alpha2);

-- 9. Recreate the index on the restored bpchar column.
CREATE INDEX idx_country_language ON public.iso_country USING btree (country_primary_language);