CREATE TABLE live_chat_messages (
    live_chat_message_id UUID PRIMARY KEY,
    room_key VARCHAR NOT NULL DEFAULT 'main',
    user_id UUID NULL REFERENCES users(user_id) ON DELETE SET NULL,
    guest_ip INET NULL,
    sender_kind SMALLINT NOT NULL,
    sender_display_name TEXT NOT NULL,
    message_body TEXT NOT NULL,
    message_created_at TIMESTAMPTZ NOT NULL,
    message_edited_at TIMESTAMPTZ NULL,
    message_deleted_at TIMESTAMPTZ NULL,
    CONSTRAINT live_chat_sender_kind_valid CHECK (sender_kind IN (0, 1)),
    CONSTRAINT live_chat_sender_identity_valid CHECK (
        (sender_kind = 0 AND guest_ip IS NOT NULL)
        OR
        (sender_kind = 1 AND user_id IS NOT NULL)
    )
);

CREATE INDEX live_chat_messages_room_created_idx
    ON live_chat_messages (room_key, message_created_at DESC);

CREATE INDEX live_chat_messages_user_created_idx
    ON live_chat_messages (user_id, message_created_at DESC);

CREATE INDEX live_chat_messages_guest_ip_created_idx
    ON live_chat_messages (guest_ip, message_created_at DESC);
