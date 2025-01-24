-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_refresh_tokens_token;

DROP INDEX IF EXISTS idx_refresh_tokens_user_id;

DROP TABLE IF EXISTS public.refresh_tokens;

DROP INDEX IF EXISTS idx_password_reset_tokens_token;

DROP INDEX IF EXISTS idx_password_reset_tokens_user_id;

DROP TABLE IF EXISTS public.password_reset_tokens;

DROP INDEX IF EXISTS idx_email_verification_tokens_token;

DROP INDEX IF EXISTS idx_email_verification_tokens_user_id;

DROP TABLE IF EXISTS public.email_verification_tokens;

ALTER TABLE public.users
DROP COLUMN IF EXISTS user_is_email_verified;
