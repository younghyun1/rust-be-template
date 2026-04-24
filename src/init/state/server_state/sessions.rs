use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::ServerState;
use crate::domain::auth::{
    role::RoleType,
    user::{User, UserInfo},
    user_roles::UserRole,
};
use crate::init::state::session::{DEFAULT_SESSION_DURATION, Session};
use crate::schema::users;

impl ServerState {
    pub async fn new_session(
        &self,
        user: &User,
        is_email_verified: bool,
        valid_for: Option<chrono::Duration>,
    ) -> anyhow::Result<Uuid> {
        let role_type = match self
            .role_for_user_or_insert_default(user.user_id, RoleType::User)
            .await
        {
            Ok(role_type) => role_type,
            Err(e) => return Err(e),
        };

        let session_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let session_duration = match valid_for {
            Some(duration) => duration,
            None => DEFAULT_SESSION_DURATION,
        };
        let expires_at = now + session_duration;
        match self
            .session_map
            .insert_async(
                session_id,
                Session {
                    session_id,
                    is_email_verified,
                    created_at: now,
                    expires_at,
                    user_id: user.user_id,
                    role_type,
                    user_language: user.user_language,
                    user_name: user.user_name.clone(),
                    user_country: user.user_country,
                },
            )
            .await
        {
            Ok(_) => (),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Failed to insert session into scc::HashMap; key already exists!"
                ));
            }
        };

        Ok(session_id)
    }

    pub async fn role_for_user(&self, user_id: Uuid) -> anyhow::Result<Option<RoleType>> {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => return Err(e),
        };
        UserRole::role_for_user(&mut conn, user_id).await
    }

    pub async fn role_for_user_or_insert_default(
        &self,
        user_id: Uuid,
        default_role_type: RoleType,
    ) -> anyhow::Result<RoleType> {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => return Err(e),
        };
        UserRole::role_for_user_or_insert_default(&mut conn, user_id, default_role_type).await
    }

    pub async fn refresh_sessions_for_user(&self, user_id: Uuid) -> anyhow::Result<usize> {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => return Err(e),
        };

        let user_info = match users::table
            .filter(users::user_id.eq(user_id))
            .select(UserInfo::as_select())
            .first::<UserInfo>(&mut conn)
            .await
            .optional()
        {
            Ok(user_info) => user_info,
            Err(e) => return Err(anyhow::anyhow!("Failed to fetch user info: {}", e)),
        };

        let user_info = match user_info {
            Some(user_info) => user_info,
            None => return Ok(0),
        };

        let role_type =
            match UserRole::role_for_user_or_insert_default(&mut conn, user_id, RoleType::User)
                .await
            {
                Ok(role_type) => role_type,
                Err(e) => return Err(e),
            };

        drop(conn);

        let mut refreshed = 0;
        self.session_map
            .iter_mut_async(|mut entry| {
                if entry.user_id == user_id {
                    entry.user_name = user_info.user_name.clone();
                    entry.user_country = user_info.user_country;
                    entry.user_language = user_info.user_language;
                    entry.is_email_verified = user_info.user_is_email_verified;
                    entry.role_type = role_type;
                    refreshed += 1;
                }
                true
            })
            .await;

        Ok(refreshed)
    }

    pub async fn get_session(&self, session_id: &Uuid) -> anyhow::Result<Session> {
        match self
            .session_map
            .read_async(session_id, |_, v| v.clone())
            .await
        {
            Some(session) => Ok(session),
            None => Err(anyhow::anyhow!("Session not found")),
        }
    }

    pub fn get_session_length(&self) -> usize {
        self.session_map.len()
    }

    pub async fn remove_session(&self, session_id: Uuid) -> anyhow::Result<(Uuid, usize)> {
        let cur_session_count = self.session_map.len();
        match self.session_map.remove_async(&session_id).await {
            Some((session_id, _)) => Ok((session_id, cur_session_count - 1)),
            None => Err(anyhow::anyhow!("Session map out of sync!")),
        }
    }

    pub async fn purge_expired_sessions(&self) -> (usize, usize) {
        let now = chrono::Utc::now();
        let (mut pruned, mut remaining): (usize, usize) = (0, 0);

        self.session_map
            .iter_mut_async(|entry| {
                if entry.expires_at < now {
                    pruned += 1;
                    let _ = entry.consume();
                } else {
                    remaining += 1;
                }
                true
            })
            .await;

        (pruned, remaining)
    }
}
