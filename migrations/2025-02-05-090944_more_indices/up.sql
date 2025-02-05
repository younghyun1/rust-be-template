CREATE INDEX idx_user_is_email_verified ON users (user_is_email_verified);

-- Index on created_at and expires_at for email verification tokens
CREATE INDEX idx_email_verification_tokens_created_at ON email_verification_tokens (email_verification_token_created_at);

CREATE INDEX idx_email_verification_tokens_expires_at ON email_verification_tokens (email_verification_token_expires_at);

-- Index on created_at and expires_at for password reset tokens
CREATE INDEX idx_password_reset_tokens_created_at ON password_reset_tokens (password_reset_token_created_at);

CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens (password_reset_token_expires_at);

-- Index on created_at and expires_at for refresh tokens
CREATE INDEX idx_refresh_tokens_issued_at ON refresh_tokens (refresh_token_issued_at);

CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens (refresh_token_expires_at);
