use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use scc::{Guard, HashMap, Queue, TreeIndex};
use serde_derive::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::{
    ban::LiveChatBan,
    message::{LIVE_CHAT_SENDER_KIND_GUEST, LiveChatMessage},
};

pub const DEFAULT_LIVE_CHAT_ROOM: &str = "main";
pub const LIVE_CHAT_CACHE_MAX_BYTES: usize = 128 * 1024 * 1024;
pub const LIVE_CHAT_BROADCAST_CAPACITY: usize = 1024;
pub const LIVE_CHAT_ABNORMAL_MESSAGE_LIMIT_PER_SECOND: u32 = 10;
const LIVE_CHAT_MESSAGE_FIXED_BYTES: usize = 256;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ChatTimelineKey {
    pub room_key: String,
    pub message_created_at_micros: i64,
    pub live_chat_message_id: Uuid,
}

impl ChatTimelineKey {
    pub fn from_message(message: &CachedChatMessage) -> Self {
        Self {
            room_key: message.room_key.clone(),
            message_created_at_micros: message.message_created_at.timestamp_micros(),
            live_chat_message_id: message.live_chat_message_id,
        }
    }
}

#[derive(Debug, Clone)]
struct ChatEvictionKey {
    live_chat_message_id: Uuid,
    timeline_key: ChatTimelineKey,
    estimated_bytes: usize,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ChatActorKey {
    User(Uuid),
    Guest(String),
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
enum LiveChatRateKey {
    User(Uuid),
    Ip(IpAddr),
}

#[derive(Debug, Clone)]
struct LiveChatRateState {
    window_started_at: DateTime<Utc>,
    count: u32,
}

#[derive(Debug, Clone)]
pub struct CachedLiveChatBan {
    pub live_chat_ban_id: Uuid,
    pub user_id: Option<Uuid>,
    pub banned_ip: Option<IpAddr>,
    pub reason: String,
    pub ban_source: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl CachedLiveChatBan {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        match self.expires_at {
            Some(expires_at) => expires_at > now,
            None => true,
        }
    }
}

impl From<LiveChatBan> for CachedLiveChatBan {
    fn from(ban: LiveChatBan) -> Self {
        Self {
            live_chat_ban_id: ban.live_chat_ban_id,
            user_id: ban.user_id,
            banned_ip: ban.banned_ip.map(|ip| ip.addr()),
            reason: ban.reason,
            ban_source: ban.ban_source,
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatActor {
    pub actor_key: ChatActorKey,
    pub sender_kind: i16,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<IpAddr>,
    pub display_name: String,
    pub country_flag: Option<String>,
    pub user_profile_picture_url: Option<String>,
}

impl ChatActor {
    pub fn guest(ip: IpAddr, country_flag: Option<String>) -> Self {
        let display_name = format!("guest@{ip}");
        Self {
            actor_key: ChatActorKey::Guest(ip.to_string()),
            sender_kind: LIVE_CHAT_SENDER_KIND_GUEST,
            user_id: None,
            guest_ip: Some(ip),
            display_name,
            country_flag,
            user_profile_picture_url: None,
        }
    }

    pub fn user(
        user_id: Uuid,
        display_name: String,
        country_flag: Option<String>,
        user_profile_picture_url: Option<String>,
    ) -> Self {
        Self {
            actor_key: ChatActorKey::User(user_id),
            sender_kind: super::message::LIVE_CHAT_SENDER_KIND_USER,
            user_id: Some(user_id),
            guest_ip: None,
            display_name,
            country_flag,
            user_profile_picture_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedChatMessage {
    pub live_chat_message_id: Uuid,
    pub room_key: String,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<IpAddr>,
    pub sender_kind: i16,
    pub sender_display_name: String,
    pub sender_country_flag: Option<String>,
    pub user_profile_picture_url: Option<String>,
    pub message_body: String,
    pub message_created_at: DateTime<Utc>,
    pub message_edited_at: Option<DateTime<Utc>>,
    pub message_deleted_at: Option<DateTime<Utc>>,
}

impl CachedChatMessage {
    pub fn estimated_bytes(&self) -> usize {
        LIVE_CHAT_MESSAGE_FIXED_BYTES
            + self.room_key.len()
            + self.sender_display_name.len()
            + match &self.sender_country_flag {
                Some(country_flag) => country_flag.len(),
                None => 0,
            }
            + match &self.user_profile_picture_url {
                Some(user_profile_picture_url) => user_profile_picture_url.len(),
                None => 0,
            }
            + self.message_body.len()
    }
}

impl From<LiveChatMessage> for CachedChatMessage {
    fn from(message: LiveChatMessage) -> Self {
        Self {
            live_chat_message_id: message.live_chat_message_id,
            room_key: message.room_key,
            user_id: message.user_id,
            guest_ip: message.guest_ip.map(|ip| ip.addr()),
            sender_kind: message.sender_kind,
            sender_display_name: message.sender_display_name,
            sender_country_flag: None,
            user_profile_picture_url: None,
            message_body: message.message_body,
            message_created_at: message.message_created_at,
            message_edited_at: message.message_edited_at,
            message_deleted_at: message.message_deleted_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingState {
    pub actor: ChatActor,
    pub room_key: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ChatConnectionState {
    pub actor: ChatActor,
    pub room_key: String,
    pub connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LiveChatServerEvent {
    Hello {
        actor: ChatActor,
        recent_messages: Vec<CachedChatMessage>,
        connected_count: u64,
    },
    Message {
        message: CachedChatMessage,
    },
    MessageAck {
        client_message_id: String,
        message: CachedChatMessage,
    },
    Typing {
        actor: ChatActor,
        is_typing: bool,
        expires_at: DateTime<Utc>,
    },
    TypingSet {
        actors: Vec<ChatActor>,
        expires_at: DateTime<Utc>,
    },
    Presence {
        connected_count: u64,
    },
    HeartbeatAck {
        nonce: String,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatCacheStats {
    pub max_bytes: usize,
    pub used_bytes: usize,
    pub message_count: usize,
    pub oldest_cached_at: Option<DateTime<Utc>>,
    pub newest_cached_at: Option<DateTime<Utc>>,
    pub active_typing_count: usize,
    pub connected_count: u64,
}

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
