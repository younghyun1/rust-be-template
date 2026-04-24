use std::collections::HashMap as StdHashMap;

use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use tracing::info;
use uuid::Uuid;

use super::ServerState;
use crate::domain::live_chat::{
    ban::LiveChatBan, cache::CachedChatMessage, message::LiveChatMessage,
};
use crate::schema::{live_chat_bans, live_chat_messages, user_profile_pictures, users};
use crate::util::time::now::tokio_now;

impl ServerState {
    pub async fn enrich_live_chat_message_flags(
        &self,
        messages: &mut [CachedChatMessage],
    ) -> anyhow::Result<()> {
        let mut user_ids = Vec::new();
        let mut seen_user_ids: StdHashMap<Uuid, ()> = StdHashMap::new();
        for message in messages.iter() {
            if message.sender_country_flag.is_some() && message.user_profile_picture_url.is_some() {
                continue;
            }
            if let Some(user_id) = message.user_id
                && !seen_user_ids.contains_key(&user_id)
            {
                seen_user_ids.insert(user_id, ());
                user_ids.push(user_id);
            }
        }

        let mut user_country_codes: StdHashMap<Uuid, i32> = StdHashMap::new();
        let mut user_profile_picture_urls: StdHashMap<Uuid, String> = StdHashMap::new();
        if !user_ids.is_empty() {
            let mut conn = self.get_conn().await?;
            let rows: Vec<(Uuid, i32)> = users::table
                .filter(users::user_id.eq_any(&user_ids))
                .select((users::user_id, users::user_country))
                .load(&mut conn)
                .await?;

            let profile_picture_rows: Vec<(Uuid, Option<String>)> = user_profile_pictures::table
                .filter(user_profile_pictures::user_id.eq_any(&user_ids))
                .distinct_on(user_profile_pictures::user_id)
                .order((
                    user_profile_pictures::user_id,
                    user_profile_pictures::user_profile_picture_updated_at.desc(),
                ))
                .select((
                    user_profile_pictures::user_id,
                    user_profile_pictures::user_profile_picture_link,
                ))
                .load(&mut conn)
                .await?;
            drop(conn);

            for (user_id, user_country) in rows {
                user_country_codes.insert(user_id, user_country);
            }

            for (user_id, user_profile_picture_url) in profile_picture_rows {
                if user_profile_picture_urls.contains_key(&user_id) {
                    continue;
                }
                if let Some(user_profile_picture_url) = user_profile_picture_url {
                    user_profile_picture_urls.insert(user_id, user_profile_picture_url);
                }
            }
        }

        let country_map = self.country_map.read().await;
        for message in messages.iter_mut() {
            let country_flag = match (message.sender_country_flag.clone(), message.user_id) {
                (Some(country_flag), _) => Some(country_flag),
                (None, Some(user_id)) => user_country_codes
                    .get(&user_id)
                    .and_then(|country_code| country_map.get_flag_by_code(*country_code)),
                (None, None) => match message.guest_ip {
                    Some(guest_ip) => self.lookup_ip_location(guest_ip).and_then(|ip_info| {
                        country_map
                            .lookup_by_alpha2(&ip_info.country_code)
                            .map(|country| country.country.country_flag.clone())
                    }),
                    None => None,
                },
            };
            message.sender_country_flag = country_flag;

            if message.user_profile_picture_url.is_none()
                && let Some(user_id) = message.user_id
                && let Some(user_profile_picture_url) = user_profile_picture_urls.get(&user_id)
            {
                message.user_profile_picture_url = Some(user_profile_picture_url.clone());
            }
        }

        Ok(())
    }

    pub async fn sync_live_chat_ban_cache(&self) -> anyhow::Result<usize> {
        let start = tokio_now();
        let now = chrono::Utc::now();
        let mut conn = self.get_conn().await?;

        let rows: Vec<LiveChatBan> = live_chat_bans::table
            .filter(
                live_chat_bans::expires_at
                    .is_null()
                    .or(live_chat_bans::expires_at.gt(now)),
            )
            .select(LiveChatBan::as_select())
            .load(&mut conn)
            .await?;

        drop(conn);

        let row_count = rows.len();
        self.live_chat_cache.sync_bans(rows).await;

        info!(
            elapsed = ?start.elapsed(),
            rows_synchronized = %row_count,
            "Synchronized live chat ban cache."
        );

        Ok(row_count)
    }

    pub async fn sync_live_chat_cache(&self) -> anyhow::Result<usize> {
        const LIVE_CHAT_STARTUP_ROW_LIMIT: i64 = 50_000;

        let start = tokio_now();
        let mut conn = self.get_conn().await?;

        let mut rows: Vec<LiveChatMessage> = live_chat_messages::table
            .select(LiveChatMessage::as_select())
            .order(live_chat_messages::message_created_at.desc())
            .limit(LIVE_CHAT_STARTUP_ROW_LIMIT)
            .load(&mut conn)
            .await?;

        drop(conn);

        self.live_chat_cache.clear_messages().await;
        rows.reverse();
        let row_count = rows.len();
        let mut cached_messages = rows
            .into_iter()
            .map(CachedChatMessage::from)
            .collect::<Vec<CachedChatMessage>>();
        self.enrich_live_chat_message_flags(&mut cached_messages)
            .await?;

        for row in cached_messages {
            self.live_chat_cache
                .append_persisted_chat_message(row)
                .await;
        }

        info!(
            elapsed = ?start.elapsed(),
            rows_synchronized = %row_count,
            "Synchronized live chat cache."
        );

        Ok(row_count)
    }
}
