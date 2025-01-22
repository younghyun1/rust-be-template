ALTER TABLE public.users
ADD COLUMN is_email_verified BOOLEAN DEFAULT FALSE NOT NULL;

CREATE TABLE public.email_verification_tokens (
    email_verification_token_id UUID DEFAULT uuid_generate_v4 () NOT NULL,
    user_id UUID NOT NULL,
    email_verification_token VARCHAR NOT NULL,
    email_verification_token_expires_at TIMESTAMPTZ NOT NULL,
    email_verification_tokens_created_at TIMESTAMPTZ DEFAULT now () NOT NULL,
    CONSTRAINT email_verification_tokens_pkey PRIMARY KEY (email_verification_token_id),
    CONSTRAINT fk_user_email_verification FOREIGN KEY (user_id) REFERENCES public.users (user_id) ON DELETE CASCADE
);
CREATE INDEX idx_email_verification_tokens_user_id ON public.email_verification_tokens (user_id);
CREATE INDEX idx_email_verification_tokens_token ON public.email_verification_tokens (email_verification_token);

CREATE TABLE public.password_reset_tokens (
    password_reset_token_id UUID DEFAULT uuid_generate_v4 () NOT NULL,
    user_id UUID NOT NULL,
    password_reset_token VARCHAR NOT NULL,
    password_reset_token_expires_at TIMESTAMPTZ NOT NULL,
    password_reset_token_created_at TIMESTAMPTZ DEFAULT now () NOT NULL,
    CONSTRAINT password_reset_tokens_pkey PRIMARY KEY (password_reset_token_id),
    CONSTRAINT fk_user_password_reset FOREIGN KEY (user_id) REFERENCES public.users (user_id) ON DELETE CASCADE
);
CREATE INDEX idx_password_reset_tokens_user_id ON public.password_reset_tokens (user_id);
CREATE INDEX idx_password_reset_tokens_token ON public.password_reset_tokens (password_reset_token);

CREATE TABLE public.refresh_tokens (
    refresh_token_id UUID DEFAULT uuid_generate_v4 () NOT NULL,
    user_id UUID NOT NULL,
    refresh_token VARCHAR NOT NULL,
    refresh_token_issued_at TIMESTAMPTZ DEFAULT now () NOT NULL,
    refresh_token_expires_at TIMESTAMPTZ NOT NULL,
    refresh_token_revoked BOOLEAN DEFAULT FALSE NOT NULL,
    CONSTRAINT refresh_tokens_pkey PRIMARY KEY (refresh_token_id),
    CONSTRAINT fk_user_refresh_token FOREIGN KEY (user_id) REFERENCES public.users (user_id) ON DELETE CASCADE
);
CREATE INDEX idx_refresh_tokens_user_id ON public.refresh_tokens (user_id);
CREATE INDEX idx_refresh_tokens_token ON public.refresh_tokens (refresh_token);
