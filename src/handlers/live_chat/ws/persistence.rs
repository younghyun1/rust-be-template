use std::sync::Arc;

use chrono::Utc;
use diesel_async::RunQueryDsl;
use tracing::error;
use uuid::Uuid;

use crate::{
    domain::live_chat::{
        ban::{LIVE_CHAT_BAN_SOURCE_ABNORMAL_MESSAGING, LiveChatBan, LiveChatBanInsertable},
        cache::{CachedChatMessage, CachedLiveChatBan, ChatActor, DEFAULT_LIVE_CHAT_ROOM},
        message::LiveChatMessageInsertable,
    },
    init::state::ServerState,
    schema::{live_chat_bans, live_chat_messages},
};

pub(super) async fn persist_message(
    state: Arc<ServerState>,
    actor: &ChatActor,
    body: String,
) -> Option<CachedChatMessage> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = ?e, "Failed to get DB connection for live chat message");
            return None;
        }
    };

    let new_message = LiveChatMessageInsertable {
        live_chat_message_id: Uuid::now_v7(),
        room_key: DEFAULT_LIVE_CHAT_ROOM.to_string(),
        user_id: actor.user_id,
        guest_ip: actor.guest_ip.map(ipnet::IpNet::from),
        sender_kind: actor.sender_kind,
        sender_display_name: actor.display_name.clone(),
        message_body: body,
        message_created_at: Utc::now(),
    };

    let persisted = diesel::insert_into(live_chat_messages::table)
        .values(new_message)
        .returning(live_chat_messages::all_columns)
        .get_result::<crate::domain::live_chat::message::LiveChatMessage>(&mut conn)
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to persist live chat message");
            e
        })
        .ok();

    drop(conn);
    persisted.map(|message| {
        let mut cached_message = CachedChatMessage::from(message);
        cached_message.sender_country_flag = actor.country_flag.clone();
        cached_message.user_profile_picture_url = actor.user_profile_picture_url.clone();
        cached_message
    })
}

pub(super) async fn persist_live_chat_ban(
    state: Arc<ServerState>,
    actor: &ChatActor,
    client_ip: std::net::IpAddr,
) -> Option<CachedLiveChatBan> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = ?e, "Failed to get DB connection for live chat ban");
            return None;
        }
    };

    let new_ban = LiveChatBanInsertable {
        live_chat_ban_id: Uuid::now_v7(),
        user_id: actor.user_id,
        banned_ip: Some(ipnet::IpNet::from(client_ip)),
        reason: "More than 10 live chat message events in one second.".to_string(),
        ban_source: LIVE_CHAT_BAN_SOURCE_ABNORMAL_MESSAGING.to_string(),
        banned_at: Utc::now(),
        expires_at: None,
    };

    let persisted = diesel::insert_into(live_chat_bans::table)
        .values(new_ban)
        .returning(live_chat_bans::all_columns)
        .get_result::<LiveChatBan>(&mut conn)
        .await
        .map_err(|e| {
            error!(error = ?e, user_id = ?actor.user_id, client_ip = %client_ip, "Failed to persist live chat ban");
            e
        })
        .ok();

    drop(conn);
    persisted.map(CachedLiveChatBan::from)
}
