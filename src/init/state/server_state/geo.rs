use std::net::IpAddr;

use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::error;
use uuid::Uuid;

use super::ServerState;
use crate::schema::user_profile_pictures;
use crate::util::geographic::ip_info_lookup::{IpInfo, lookup_ip_location_from_map};

impl ServerState {
    pub fn lookup_ip_location(&self, ip: IpAddr) -> Option<IpInfo> {
        lookup_ip_location_from_map(&self.geo_ip_db, ip)
    }

    pub async fn country_flag_for_country_code(&self, country_code: i32) -> Option<String> {
        let country_map = self.country_map.read().await;
        country_map.get_flag_by_code(country_code)
    }

    pub async fn country_flag_for_ip(&self, ip: IpAddr) -> Option<String> {
        let ip_info = match self.lookup_ip_location(ip) {
            Some(ip_info) => ip_info,
            None => return None,
        };
        let country_map = self.country_map.read().await;
        country_map
            .lookup_by_alpha2(&ip_info.country_code)
            .map(|country| country.country.country_flag.clone())
    }

    pub async fn latest_user_profile_picture_url(&self, user_id: Uuid) -> Option<String> {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = ?e, user_id = %user_id, "Failed to get DB connection for user profile picture lookup");
                return None;
            }
        };

        let user_profile_picture_url = user_profile_pictures::table
            .filter(user_profile_pictures::user_id.eq(user_id))
            .order(user_profile_pictures::user_profile_picture_updated_at.desc())
            .select(user_profile_pictures::user_profile_picture_link)
            .first::<Option<String>>(&mut conn)
            .await
            .optional()
            .map_err(|e| {
                error!(error = ?e, user_id = %user_id, "Failed to query latest user profile picture");
                e
            })
            .ok()
            .flatten()
            .flatten();

        drop(conn);
        user_profile_picture_url
    }
}
