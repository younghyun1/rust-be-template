use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use scc::{Guard, HashMap, Queue, TreeIndex};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::domain::live_chat::ban::LiveChatBan;

mod actor;
mod ban;
mod event;
mod message;
mod rate;

pub use actor::{ChatActor, ChatActorKey};
pub use ban::CachedLiveChatBan;
pub use event::{ChatConnectionState, LiveChatCacheStats, LiveChatServerEvent, TypingState};
pub use message::{CachedChatMessage, ChatTimelineKey};

use self::{
    message::ChatEvictionKey,
    rate::{LiveChatRateKey, LiveChatRateState},
};

pub const DEFAULT_LIVE_CHAT_ROOM: &str = "main";
pub const LIVE_CHAT_CACHE_MAX_BYTES: usize = 128 * 1024 * 1024;
pub const LIVE_CHAT_BROADCAST_CAPACITY: usize = 1024;
pub const LIVE_CHAT_ABNORMAL_MESSAGE_LIMIT_PER_SECOND: u32 = 10;
const LIVE_CHAT_MESSAGE_FIXED_BYTES: usize = 256;

pub struct LiveChatCache {
    messages_by_id: HashMap<Uuid, Arc<CachedChatMessage>>,
    timeline: TreeIndex<ChatTimelineKey, Uuid>,
    eviction_queue: Queue<ChatEvictionKey>,
    typing_by_actor: HashMap<ChatActorKey, TypingState>,
    connected_clients: HashMap<Uuid, ChatConnectionState>,
    bans_by_user: HashMap<Uuid, CachedLiveChatBan>,
    bans_by_ip: HashMap<IpAddr, CachedLiveChatBan>,
    message_rate_by_key: HashMap<LiveChatRateKey, LiveChatRateState>,
    total_bytes: AtomicUsize,
    message_count: AtomicUsize,
    connected_count: AtomicU64,
    max_bytes: usize,
    broadcast_tx: broadcast::Sender<LiveChatServerEvent>,
}

impl LiveChatCache {
    pub fn new(max_bytes: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(LIVE_CHAT_BROADCAST_CAPACITY);
        Self {
            messages_by_id: HashMap::new(),
            timeline: TreeIndex::new(),
            eviction_queue: Queue::new(),
            typing_by_actor: HashMap::new(),
            connected_clients: HashMap::new(),
            bans_by_user: HashMap::new(),
            bans_by_ip: HashMap::new(),
            message_rate_by_key: HashMap::new(),
            total_bytes: AtomicUsize::new(0),
            message_count: AtomicUsize::new(0),
            connected_count: AtomicU64::new(0),
            max_bytes,
            broadcast_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LiveChatServerEvent> {
        self.broadcast_tx.subscribe()
    }

    pub fn broadcast(&self, event: LiveChatServerEvent) {
        let _ = self.broadcast_tx.send(event);
    }

    pub async fn sync_bans(&self, bans: Vec<LiveChatBan>) {
        self.bans_by_user.clear_async().await;
        self.bans_by_ip.clear_async().await;
        let now = Utc::now();

        for ban in bans {
            let cached_ban = CachedLiveChatBan::from(ban);
            if cached_ban.is_active(now) {
                self.cache_ban(cached_ban).await;
            }
        }
    }

    pub async fn cache_ban(&self, ban: CachedLiveChatBan) {
        if let Some(user_id) = ban.user_id {
            let _ = self.bans_by_user.upsert_async(user_id, ban.clone()).await;
        }

        if let Some(ip) = ban.banned_ip {
            let _ = self.bans_by_ip.upsert_async(ip, ban).await;
        }
    }

    pub async fn is_banned(&self, user_id: Option<Uuid>, ip: IpAddr) -> bool {
        let now = Utc::now();

        if let Some(user_id) = user_id
            && let Some(is_active) = self
                .bans_by_user
                .read_async(&user_id, |_, ban| ban.is_active(now))
                .await
        {
            if is_active {
                return true;
            }
            let _ = self.bans_by_user.remove_async(&user_id).await;
        }

        if let Some(is_active) = self
            .bans_by_ip
            .read_async(&ip, |_, ban| ban.is_active(now))
            .await
        {
            if is_active {
                return true;
            }
            let _ = self.bans_by_ip.remove_async(&ip).await;
        }

        false
    }

    pub async fn record_message_attempt(
        &self,
        user_id: Option<Uuid>,
        ip: IpAddr,
        now: DateTime<Utc>,
    ) -> bool {
        let ip_abnormal = self
            .record_message_attempt_for_key(LiveChatRateKey::Ip(ip), now)
            .await;
        let user_abnormal = match user_id {
            Some(user_id) => {
                self.record_message_attempt_for_key(LiveChatRateKey::User(user_id), now)
                    .await
            }
            None => false,
        };

        ip_abnormal || user_abnormal
    }

    async fn record_message_attempt_for_key(
        &self,
        key: LiveChatRateKey,
        now: DateTime<Utc>,
    ) -> bool {
        let mut is_abnormal = false;
        if self
            .message_rate_by_key
            .update_async(&key, |_, state| {
                if now.signed_duration_since(state.window_started_at) < ChronoDuration::seconds(1) {
                    state.count = state.count.saturating_add(1);
                } else {
                    state.window_started_at = now;
                    state.count = 1;
                }
                is_abnormal = state.count > LIVE_CHAT_ABNORMAL_MESSAGE_LIMIT_PER_SECOND;
            })
            .await
            .is_none()
        {
            match self
                .message_rate_by_key
                .insert_async(
                    key,
                    LiveChatRateState {
                        window_started_at: now,
                        count: 1,
                    },
                )
                .await
            {
                Ok(_) => {}
                Err((key, _state)) => {
                    let mut raced_is_abnormal = false;
                    let _ = self
                        .message_rate_by_key
                        .update_async(&key, |_, state| {
                            if now.signed_duration_since(state.window_started_at)
                                < ChronoDuration::seconds(1)
                            {
                                state.count = state.count.saturating_add(1);
                            } else {
                                state.window_started_at = now;
                                state.count = 1;
                            }
                            raced_is_abnormal =
                                state.count > LIVE_CHAT_ABNORMAL_MESSAGE_LIMIT_PER_SECOND;
                        })
                        .await;
                    return raced_is_abnormal;
                }
            }
        }

        is_abnormal
    }

    pub async fn clear_messages(&self) {
        self.messages_by_id.clear_async().await;
        self.timeline.clear();
        while self.eviction_queue.pop().is_some() {}
        self.total_bytes.store(0, Ordering::SeqCst);
        self.message_count.store(0, Ordering::SeqCst);
    }

    pub async fn append_persisted_chat_message(&self, message: CachedChatMessage) {
        let estimated_bytes = message.estimated_bytes();
        let timeline_key = ChatTimelineKey::from_message(&message);
        let message_id = message.live_chat_message_id;
        let message = Arc::new(message);

        if let Some(previous) = self.messages_by_id.upsert_async(message_id, message).await {
            self.total_bytes
                .fetch_sub(previous.estimated_bytes(), Ordering::SeqCst);
            self.message_count.fetch_sub(1, Ordering::SeqCst);
        }

        let _ = self
            .timeline
            .insert_async(timeline_key.clone(), message_id)
            .await;
        self.eviction_queue.push(ChatEvictionKey {
            live_chat_message_id: message_id,
            timeline_key,
            estimated_bytes,
        });
        self.total_bytes
            .fetch_add(estimated_bytes, Ordering::SeqCst);
        self.message_count.fetch_add(1, Ordering::SeqCst);
        self.evict_over_budget().await;
    }

    async fn evict_over_budget(&self) {
        while self.total_bytes.load(Ordering::SeqCst) > self.max_bytes {
            let popped = match self.eviction_queue.pop() {
                Some(entry) => entry,
                None => return,
            };
            let eviction_key = (**popped).clone();
            if self
                .messages_by_id
                .remove_async(&eviction_key.live_chat_message_id)
                .await
                .is_some()
            {
                let _ = self.timeline.remove_async(&eviction_key.timeline_key).await;
                self.total_bytes
                    .fetch_sub(eviction_key.estimated_bytes, Ordering::SeqCst);
                self.message_count.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    pub async fn get_recent_chat_messages(&self, limit: usize) -> Vec<CachedChatMessage> {
        let ids = {
            let guard = Guard::new();
            let mut ids = Vec::with_capacity(limit);
            let mut iter = self.timeline.iter(&guard);
            while ids.len() < limit {
                match iter.next_back() {
                    Some((_, message_id)) => ids.push(*message_id),
                    None => break,
                }
            }
            ids.reverse();
            ids
        };

        let mut messages = Vec::with_capacity(ids.len());
        for message_id in ids {
            if let Some(message) = self
                .messages_by_id
                .read_async(&message_id, |_, message| (**message).clone())
                .await
            {
                messages.push(message);
            }
        }
        messages
    }

    pub async fn get_chat_messages_before(
        &self,
        before: ChatTimelineKey,
        limit: usize,
    ) -> Vec<CachedChatMessage> {
        let ids = {
            let guard = Guard::new();
            let mut ids = Vec::with_capacity(limit);
            let mut range = self.timeline.range(..before, &guard);
            while ids.len() < limit {
                match range.next_back() {
                    Some((_, message_id)) => ids.push(*message_id),
                    None => break,
                }
            }
            ids.reverse();
            ids
        };

        let mut messages = Vec::with_capacity(ids.len());
        for message_id in ids {
            if let Some(message) = self
                .messages_by_id
                .read_async(&message_id, |_, message| (**message).clone())
                .await
            {
                messages.push(message);
            }
        }
        messages
    }

    pub async fn get_timeline_key_for_message(&self, message_id: Uuid) -> Option<ChatTimelineKey> {
        self.messages_by_id
            .read_async(&message_id, |_, message| {
                ChatTimelineKey::from_message(message)
            })
            .await
    }

    pub async fn set_typing(&self, state: TypingState) -> bool {
        let actor_key = state.actor.actor_key.clone();
        self.typing_by_actor
            .upsert_async(actor_key, state)
            .await
            .is_none()
    }

    pub async fn clear_typing(&self, actor_key: &ChatActorKey) -> bool {
        self.typing_by_actor.remove_async(actor_key).await.is_some()
    }

    pub async fn clear_expired_typing(&self, now: DateTime<Utc>) {
        self.typing_by_actor
            .retain_async(|_, typing_state| typing_state.expires_at > now)
            .await;
    }

    pub async fn active_typing_actors(&self, now: DateTime<Utc>) -> Vec<ChatActor> {
        self.clear_expired_typing(now).await;
        let mut actors = Vec::with_capacity(self.typing_by_actor.len());
        self.typing_by_actor
            .iter_async(|_, typing_state| {
                actors.push(typing_state.actor.clone());
                true
            })
            .await;
        actors
    }

    pub async fn register_connection(
        &self,
        connection_id: Uuid,
        connection_state: ChatConnectionState,
    ) {
        let _ = self
            .connected_clients
            .insert_async(connection_id, connection_state)
            .await;
        self.connected_count.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn unregister_connection(&self, connection_id: Uuid) {
        if self
            .connected_clients
            .remove_async(&connection_id)
            .await
            .is_some()
        {
            self.connected_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn connected_count(&self) -> u64 {
        self.connected_count.load(Ordering::SeqCst)
    }

    pub async fn stats(&self) -> LiveChatCacheStats {
        let guard = Guard::new();
        let mut iter = self.timeline.iter(&guard);
        let oldest_cached_at = iter.next().and_then(|(_, message_id)| {
            self.messages_by_id
                .read_sync(message_id, |_, message| message.message_created_at)
        });
        let newest_cached_at = match iter.next_back().and_then(|(_, message_id)| {
            self.messages_by_id
                .read_sync(message_id, |_, message| message.message_created_at)
        }) {
            Some(value) => Some(value),
            None => oldest_cached_at,
        };

        LiveChatCacheStats {
            max_bytes: self.max_bytes,
            used_bytes: self.total_bytes.load(Ordering::SeqCst),
            message_count: self.message_count.load(Ordering::SeqCst),
            oldest_cached_at,
            newest_cached_at,
            active_typing_count: self.typing_by_actor.len(),
            connected_count: self.connected_count(),
        }
    }
}

impl Default for LiveChatCache {
    fn default() -> Self {
        Self::new(LIVE_CHAT_CACHE_MAX_BYTES)
    }
}
