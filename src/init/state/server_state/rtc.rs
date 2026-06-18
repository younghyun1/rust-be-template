//! SFU room registry and call-record persistence on [`ServerState`].
//!
//! The webrtc mechanics live in `domain::live_chat::rtc`; this module owns the
//! `rtc_rooms` registry lifecycle (create on first join, drop when empty) and
//! the `live_chat_calls` / `live_chat_call_participants` rows.

use std::sync::Arc;

use chrono::Utc;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::error;
use uuid::Uuid;

use super::ServerState;
use crate::domain::live_chat::cache::ChatActor;
use crate::domain::live_chat::call::{LiveChatCallInsertable, LiveChatCallParticipantInsertable};
use crate::domain::live_chat::rtc::{RtcEngine, RtcRoom, RtcRoomAcquire};
use crate::schema::{live_chat_call_participants, live_chat_calls};

impl ServerState {
    /// Whether the SFU is available.
    pub fn rtc_enabled(&self) -> bool {
        self.rtc_engine.is_some()
    }

    /// Clone of the shared SFU engine, if enabled.
    pub fn rtc_engine(&self) -> Option<Arc<RtcEngine>> {
        self.rtc_engine.clone()
    }

    /// Configured maximum participants per room call.
    pub fn rtc_max_participants(&self) -> usize {
        self.rtc_config.max_participants
    }

    /// Acquire a room slot for a join: get-or-create the room for `room_key` and
    /// atomically reserve a participant slot. The reservation enforces the cap
    /// and keeps the room alive against concurrent GC; the caller must
    /// `release_slot()` on every failure path and on leave. Returns `Full` at
    /// capacity and `Unavailable` when the SFU is off or the call row cannot open.
    pub async fn acquire_rtc_room(&self, room_key: &str) -> RtcRoomAcquire {
        if self.rtc_engine.is_none() {
            return RtcRoomAcquire::Unavailable;
        }
        let max_participants = self.rtc_config.max_participants;

        // Bounded retry: an existing room can be GC'd concurrently (marked
        // removed) between our read and our reservation; on that race we drop the
        // reservation and try again, creating a fresh room if needed.
        for _ in 0..8 {
            if let Some(room) = self
                .rtc_rooms
                .read_async(room_key, |_, room| room.clone())
                .await
            {
                if !room.try_reserve() {
                    return RtcRoomAcquire::Full;
                }
                if room.is_removed() {
                    room.release_slot();
                    continue;
                }
                return RtcRoomAcquire::Acquired(room);
            }

            let call_id = match self.open_call_row(room_key).await {
                Some(call_id) => call_id,
                None => return RtcRoomAcquire::Unavailable,
            };
            let room = RtcRoom::new(
                room_key.to_string(),
                call_id,
                self.live_chat_cache.broadcast_sender(),
                max_participants,
            );
            // Reserve our own first slot on the fresh room before publishing it.
            let _ = room.try_reserve();
            match self
                .rtc_rooms
                .insert_async(room_key.to_string(), room.clone())
                .await
            {
                Ok(_) => return RtcRoomAcquire::Acquired(room),
                Err(_) => {
                    // Lost the create race: release and close our orphan row, retry.
                    room.release_slot();
                    self.close_call_row(call_id).await;
                    continue;
                }
            }
        }
        RtcRoomAcquire::Unavailable
    }

    /// Remove the room for `room_key` if it has no reserved slots, closing its
    /// call row. Occupancy (not peer count) is the authority, so an in-flight
    /// join that has reserved a slot is never GC'd out from under.
    pub async fn remove_rtc_room_if_empty(&self, room_key: &str) {
        let is_empty = self
            .rtc_rooms
            .read_async(room_key, |_, room| room.occupancy() == 0)
            .await
            .unwrap_or(false);
        if !is_empty {
            return;
        }

        if let Some((_, room)) = self.rtc_rooms.remove_async(room_key).await {
            if room.occupancy() == 0 {
                room.mark_removed();
                self.close_call_row(room.call_id).await;
            } else {
                // A join reserved a slot between the check and the removal; restore.
                let _ = self
                    .rtc_rooms
                    .insert_async(room_key.to_string(), room)
                    .await;
            }
        }
    }

    /// Drop all unoccupied rooms (called from the periodic prune job), closing
    /// their call rows. Keeps `rtc_rooms` bounded to rooms with live activity.
    pub async fn prune_empty_rtc_rooms(&self) {
        let mut empty_keys = Vec::new();
        self.rtc_rooms
            .iter_async(|key, room| {
                if room.occupancy() == 0 {
                    empty_keys.push(key.clone());
                }
                true
            })
            .await;

        for key in empty_keys {
            if let Some((_, room)) = self.rtc_rooms.remove_async(&key).await {
                if room.occupancy() == 0 {
                    room.mark_removed();
                    self.close_call_row(room.call_id).await;
                } else {
                    let _ = self.rtc_rooms.insert_async(key, room).await;
                }
            }
        }
    }

    /// Insert a participant row for a join. Returns the participant id used to
    /// close the row on leave.
    pub async fn record_call_participant_join(
        &self,
        call_id: Uuid,
        actor: &ChatActor,
        had_audio: bool,
        had_video: bool,
    ) -> Option<Uuid> {
        let participant_id = Uuid::now_v7();
        let row = LiveChatCallParticipantInsertable {
            live_chat_call_participant_id: participant_id,
            live_chat_call_id: call_id,
            user_id: actor.user_id,
            guest_ip: actor.guest_ip.map(ipnet::IpNet::from),
            participant_sender_kind: actor.sender_kind,
            participant_display_name: actor.display_name.clone(),
            participant_joined_at: Utc::now(),
            participant_had_audio: had_audio,
            participant_had_video: had_video,
        };

        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = ?e, "Failed to get DB connection for call participant join");
                return None;
            }
        };
        let inserted = diesel::insert_into(live_chat_call_participants::table)
            .values(row)
            .execute(&mut conn)
            .await
            .map_err(|e| error!(error = ?e, "Failed to persist call participant join"))
            .ok();
        drop(conn);
        inserted.map(|_| participant_id)
    }

    /// Stamp a participant row's `participant_left_at` on leave.
    pub async fn record_call_participant_leave(&self, participant_id: Uuid) {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = ?e, "Failed to get DB connection for call participant leave");
                return;
            }
        };
        let _ = diesel::update(
            live_chat_call_participants::table.filter(
                live_chat_call_participants::live_chat_call_participant_id
                    .eq(participant_id)
                    .and(live_chat_call_participants::participant_left_at.is_null()),
            ),
        )
        .set(live_chat_call_participants::participant_left_at.eq(Utc::now()))
        .execute(&mut conn)
        .await
        .map_err(|e| error!(error = ?e, participant_id = %participant_id, "Failed to stamp call participant leave"));
        drop(conn);
    }

    async fn open_call_row(&self, room_key: &str) -> Option<Uuid> {
        let call_id = Uuid::now_v7();
        let new_call = LiveChatCallInsertable {
            live_chat_call_id: call_id,
            room_key: room_key.to_string(),
            call_started_at: Utc::now(),
        };

        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = ?e, "Failed to get DB connection to open call row");
                return None;
            }
        };
        let inserted = diesel::insert_into(live_chat_calls::table)
            .values(new_call)
            .execute(&mut conn)
            .await
            .map_err(|e| error!(error = ?e, "Failed to open live chat call row"))
            .ok();
        drop(conn);
        inserted.map(|_| call_id)
    }

    async fn close_call_row(&self, call_id: Uuid) {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = ?e, "Failed to get DB connection to close call row");
                return;
            }
        };
        let _ = diesel::update(
            live_chat_calls::table.filter(
                live_chat_calls::live_chat_call_id
                    .eq(call_id)
                    .and(live_chat_calls::call_ended_at.is_null()),
            ),
        )
        .set(live_chat_calls::call_ended_at.eq(Utc::now()))
        .execute(&mut conn)
        .await
        .map_err(|e| error!(error = ?e, call_id = %call_id, "Failed to close live chat call row"));
        drop(conn);
    }
}
