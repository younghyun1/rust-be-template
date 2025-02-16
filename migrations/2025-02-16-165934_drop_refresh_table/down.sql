-- This file should undo anything in `up.sql`
CREATE TABLE public.refresh_tokens (
    refresh_token_id UUID DEFAULT uuid_generate_v4 () NOT NULL,
    user_id UUID NOT NULL,
    refresh_token UUID NOT NULL,
    refresh_token_issued_at TIMESTAMPTZ DEFAULT now () NOT NULL,
    refresh_token_expires_at TIMESTAMPTZ NOT NULL,
    refresh_token_revoked BOOLEAN DEFAULT FALSE NOT NULL,
    CONSTRAINT refresh_tokens_pkey PRIMARY KEY (refresh_token_id),
    CONSTRAINT fk_user_refresh_token FOREIGN KEY (user_id) REFERENCES public.users (user_id) ON DELETE CASCADE
);

CREATE INDEX idx_refresh_tokens_user_id ON public.refresh_tokens (user_id);

CREATE INDEX idx_refresh_tokens_token ON public.refresh_tokens (refresh_token);
