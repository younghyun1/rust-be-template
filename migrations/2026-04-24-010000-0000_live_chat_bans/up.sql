CREATE TABLE live_chat_bans (
    live_chat_ban_id UUID PRIMARY KEY,
    user_id UUID NULL REFERENCES users(user_id) ON DELETE SET NULL,
    banned_ip INET NULL,
    reason TEXT NOT NULL,
    ban_source VARCHAR NOT NULL,
    banned_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NULL,
    CONSTRAINT live_chat_ban_subject_valid CHECK (
        user_id IS NOT NULL
        OR
        banned_ip IS NOT NULL
    )
);

CREATE INDEX live_chat_bans_user_idx
    ON live_chat_bans (user_id)
    WHERE user_id IS NOT NULL;

CREATE INDEX live_chat_bans_banned_ip_idx
    ON live_chat_bans (banned_ip)
    WHERE banned_ip IS NOT NULL;

CREATE INDEX live_chat_bans_active_idx
    ON live_chat_bans (expires_at);
