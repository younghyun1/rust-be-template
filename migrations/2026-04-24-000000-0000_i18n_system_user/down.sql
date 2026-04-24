DELETE FROM public.users
WHERE user_id = '00000000-0000-0000-0000-000000000000'::uuid
  AND user_email = 'system@localhost'
  AND NOT EXISTS (
      SELECT 1
      FROM public.i18n_strings
      WHERE i18n_string_created_by = '00000000-0000-0000-0000-000000000000'::uuid
         OR i18n_string_updated_by = '00000000-0000-0000-0000-000000000000'::uuid
  );
