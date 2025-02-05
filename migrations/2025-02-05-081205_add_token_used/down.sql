-- This file should undo anything in `up.sql`
ALTER TABLE public.email_verification_tokens
DROP COLUMN IF EXISTS email_verification_token_used_at;

DROP INDEX IF EXISTS public.idx_email_verification_tokens_used_at;

ALTER TABLE public.password_reset_tokens
DROP COLUMN IF EXISTS password_reset_token_used_at;

DROP INDEX IF EXISTS public.idx_password_reset_tokens_used_at;

ALTER TABLE public.refresh_tokens
DROP COLUMN IF EXISTS refresh_token_used_at;

DROP INDEX IF EXISTS public.idx_refresh_tokens_used_at;