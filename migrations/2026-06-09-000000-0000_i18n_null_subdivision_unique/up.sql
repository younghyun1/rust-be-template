-- The table-level UNIQUE (reference_key, subdivision_code, country_code, language_code)
-- does not constrain rows where subdivision_code IS NULL, because NULLs are distinct
-- under a standard UNIQUE constraint. The file-backed UI text sync writes such rows
-- (subdivision_code = NULL), so concurrent syncs could insert duplicates.
--
-- 1) Collapse any existing NULL-subdivision duplicates, keeping the most recently
--    updated row per logical key.
DELETE FROM public.i18n_strings t
USING (
    SELECT
        i18n_string_id,
        ROW_NUMBER() OVER (
            PARTITION BY
                i18n_string_reference_key,
                i18n_string_country_code,
                i18n_string_language_code
            ORDER BY
                i18n_string_updated_at DESC,
                i18n_string_created_at DESC,
                i18n_string_id DESC
        ) AS rn
    FROM public.i18n_strings
    WHERE i18n_string_country_subdivision_code IS NULL
) dups
WHERE t.i18n_string_id = dups.i18n_string_id
  AND dups.rn > 1;

-- 2) Enforce uniqueness over the NULL-subdivision rows so future syncs can upsert
--    against this partial index instead of racing update-then-insert.
CREATE UNIQUE INDEX IF NOT EXISTS i18n_strings_ui_null_subdivision_uq
    ON public.i18n_strings (
        i18n_string_reference_key,
        i18n_string_country_code,
        i18n_string_language_code
    )
    WHERE i18n_string_country_subdivision_code IS NULL;
