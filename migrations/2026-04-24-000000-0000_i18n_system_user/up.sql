INSERT INTO public.users (
    user_id,
    user_name,
    user_email,
    user_password_hash,
    user_created_at,
    user_updated_at,
    user_is_email_verified,
    user_country,
    user_language,
    user_subdivision
)
VALUES (
    '00000000-0000-0000-0000-000000000000'::uuid,
    'system',
    'system@localhost',
    'system',
    now(),
    now(),
    TRUE,
    840,
    41,
    NULL
)
ON CONFLICT (user_id) DO NOTHING;
