//! Diesel models for persisted call records. A call row spans from the first
//! join to an empty room until the room empties; participant rows record each
//! join/leave and whether the participant published audio/video. No media is
//! recorded.

use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use uuid::Uuid;

use crate::schema::{live_chat_call_participants, live_chat_calls};

#[derive(Debug, Clone, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = live_chat_calls)]
pub struct LiveChatCall {
    pub live_chat_call_id: Uuid,
    pub room_key: String,
    pub call_started_at: DateTime<Utc>,
    pub call_ended_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = live_chat_calls)]
pub struct LiveChatCallInsertable {
    pub live_chat_call_id: Uuid,
    pub room_key: String,
    pub call_started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = live_chat_call_participants)]
pub struct LiveChatCallParticipant {
    pub live_chat_call_participant_id: Uuid,
    pub live_chat_call_id: Uuid,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<ipnet::IpNet>,
    pub participant_sender_kind: i16,
    pub participant_display_name: String,
    pub participant_joined_at: DateTime<Utc>,
    pub participant_left_at: Option<DateTime<Utc>>,
    pub participant_had_audio: bool,
    pub participant_had_video: bool,
}

#[derive(Insertable)]
#[diesel(table_name = live_chat_call_participants)]
pub struct LiveChatCallParticipantInsertable {
    pub live_chat_call_participant_id: Uuid,
    pub live_chat_call_id: Uuid,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<ipnet::IpNet>,
    pub participant_sender_kind: i16,
    pub participant_display_name: String,
    pub participant_joined_at: DateTime<Utc>,
    pub participant_had_audio: bool,
    pub participant_had_video: bool,
}
