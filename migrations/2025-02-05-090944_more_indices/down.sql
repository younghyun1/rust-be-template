-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_user_is_email_verified;

DROP INDEX IF EXISTS idx_email_verification_tokens_created_at;

DROP INDEX IF EXISTS idx_email_verification_tokens_expires_at;

DROP INDEX IF EXISTS idx_password_reset_tokens_created_at;

DROP INDEX IF EXISTS idx_password_reset_tokens_expires_at;

DROP INDEX IF EXISTS idx_refresh_tokens_issued_at;

DROP INDEX IF EXISTS idx_refresh_tokens_expires_at;
