DROP INDEX IF EXISTS public.i18n_strings_i18n_string_updated_by_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_updated_at_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_reference_key_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_language_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_created_by_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_created_at_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_country_subdivision_code_idx;

DROP INDEX IF EXISTS public.i18n_strings_i18n_string_country_code_idx;

DROP TABLE IF EXISTS public.i18n_strings CASCADE;
