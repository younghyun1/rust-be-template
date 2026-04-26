use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use tracing::info;
use uuid::Uuid;

use crate::{
    domain::live_chat::cache::{
        ChatActor, DEFAULT_LIVE_CHAT_ROOM, LiveChatServerEvent, TypingState,
    },
    init::state::ServerState,
};

use super::LIVE_CHAT_TYPING_TTL_SECONDS;

pub(super) async fn handle_typing(state: Arc<ServerState>, actor: ChatActor, is_typing: bool) {
    let expires_at = Utc::now() + ChronoDuration::seconds(LIVE_CHAT_TYPING_TTL_SECONDS);
    let changed = if is_typing {
        state
            .live_chat_cache
            .set_typing(TypingState {
                actor: actor.clone(),
                room_key: DEFAULT_LIVE_CHAT_ROOM.to_string(),
                expires_at,
            })
            .await
    } else {
        state.live_chat_cache.clear_typing(&actor.actor_key).await
    };

    if is_typing || changed {
        broadcast_typing_set(state, expires_at).await;
    }
}

pub(super) async fn broadcast_typing_set(
    state: Arc<ServerState>,
    expires_at: chrono::DateTime<Utc>,
) {
    let actors = state.live_chat_cache.active_typing_actors(Utc::now()).await;
    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::TypingSet { actors, expires_at });
}

pub(super) async fn cleanup_live_chat_connection(
    state: Arc<ServerState>,
    connection_id: Uuid,
    actor: &ChatActor,
) {
    state
        .live_chat_cache
        .unregister_connection(connection_id)
        .await;
    let typing_changed = state.live_chat_cache.clear_typing(&actor.actor_key).await;
    if typing_changed {
        let expires_at = Utc::now() + ChronoDuration::seconds(LIVE_CHAT_TYPING_TTL_SECONDS);
        broadcast_typing_set(state.clone(), expires_at).await;
    }
    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Presence {
            connected_count: state.live_chat_cache.connected_count(),
        });
    info!(connection_id = %connection_id, "Live chat WebSocket disconnected");
}
