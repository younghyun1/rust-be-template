CREATE TABLE live_chat_calls (
    live_chat_call_id UUID PRIMARY KEY,
    room_key VARCHAR NOT NULL DEFAULT 'main',
    call_started_at TIMESTAMPTZ NOT NULL,
    call_ended_at TIMESTAMPTZ NULL
);

CREATE INDEX live_chat_calls_room_started_idx
    ON live_chat_calls (room_key, call_started_at DESC);
CREATE INDEX live_chat_calls_active_idx
    ON live_chat_calls (call_ended_at)
    WHERE call_ended_at IS NULL;

CREATE TABLE live_chat_call_participants (
    live_chat_call_participant_id UUID PRIMARY KEY,
    live_chat_call_id UUID NOT NULL REFERENCES live_chat_calls(live_chat_call_id) ON DELETE CASCADE,
    user_id UUID NULL REFERENCES users(user_id) ON DELETE SET NULL,
    guest_ip INET NULL,
    participant_sender_kind SMALLINT NOT NULL,
    participant_display_name TEXT NOT NULL,
    participant_joined_at TIMESTAMPTZ NOT NULL,
    participant_left_at TIMESTAMPTZ NULL,
    participant_had_audio BOOLEAN NOT NULL DEFAULT FALSE,
    participant_had_video BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT live_chat_call_participant_kind_valid CHECK (participant_sender_kind IN (0, 1)),
    CONSTRAINT live_chat_call_participant_identity_valid CHECK (
        (participant_sender_kind = 0 AND guest_ip IS NOT NULL)
        OR
        (participant_sender_kind = 1 AND user_id IS NOT NULL)
    )
);

CREATE INDEX live_chat_call_participants_call_idx
    ON live_chat_call_participants (live_chat_call_id);
CREATE INDEX live_chat_call_participants_user_idx
    ON live_chat_call_participants (user_id, participant_joined_at DESC)
    WHERE user_id IS NOT NULL;
CREATE INDEX live_chat_call_participants_guest_ip_idx
    ON live_chat_call_participants (guest_ip, participant_joined_at DESC)
    WHERE guest_ip IS NOT NULL;
