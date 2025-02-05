-- Your SQL goes here
ALTER TABLE public.email_verification_tokens
ADD COLUMN email_verification_token_used_at TIMESTAMPTZ;

CREATE INDEX idx_email_verification_tokens_used_at ON public.email_verification_tokens (email_verification_token_used_at);

ALTER TABLE public.password_reset_tokens
ADD COLUMN password_reset_token_used_at TIMESTAMPTZ;

CREATE INDEX idx_password_reset_tokens_used_at ON public.password_reset_tokens (password_reset_token_used_at);

ALTER TABLE public.refresh_tokens
ADD COLUMN refresh_token_used_at TIMESTAMPTZ;

CREATE INDEX idx_refresh_tokens_used_at ON public.refresh_tokens (refresh_token_used_at);
