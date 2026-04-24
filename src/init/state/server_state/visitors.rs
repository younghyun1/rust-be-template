use std::collections::HashMap as StdHashMap;
use std::net::IpAddr;

use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use scc::hash_map::Entry;
use tracing::{info, warn};

use super::{ServerState, VisitorLogBatch, VisitorLogKey};
use crate::domain::geo::visitation_data::NewVisitationData;
use crate::util::time::now::tokio_now;

impl ServerState {
    pub async fn sync_visitor_board_data(&self) -> anyhow::Result<usize> {
        use crate::schema::visitation_data::dsl as vdsl;

        let start = tokio_now();
        let mut conn = self.get_conn().await?;

        let visits: Vec<(f64, f64)> = vdsl::visitation_data
            .select((vdsl::latitude, vdsl::longitude))
            .load::<(f64, f64)>(&mut conn)
            .await?;

        let mut visit_counts = std::collections::HashMap::<([u8; 8], [u8; 8]), u64>::new();

        for (latitude, longitude) in visits.iter().copied() {
            let lat_bytes = latitude.to_be_bytes();
            let long_bytes = longitude.to_be_bytes();
            let key = (lat_bytes, long_bytes);
            *visit_counts.entry(key).or_insert(0) += 1;
        }

        for (key, count) in visit_counts {
            let _ = self.visitor_board_map.insert_async(key, count).await;
        }

        let num_rows = visits.len();

        info!(elapsed = ?start.elapsed(), rows_synchronized = %num_rows, "Synchronized visitor board data.");
        Ok(num_rows)
    }

    pub async fn enqueue_visitor_log(&self, inp_ip: Option<IpAddr>) {
        let ip = match inp_ip {
            Some(ip) => ip,
            None => return,
        };

        let ip_info = match self.lookup_ip_location(ip) {
            Some(info) => info,
            None => {
                warn!(ip = %ip, "Failed to look up IP location for visitor log");
                return;
            }
        };

        let city_lat = ip_info.latitude;
        let city_lon = ip_info.longitude;
        let latitude_bytes = city_lat.to_be_bytes();
        let longitude_bytes = city_lon.to_be_bytes();
        let board_key = (latitude_bytes, longitude_bytes);

        match self.visitor_board_map.entry_async(board_key).await {
            Entry::Occupied(mut occ) => {
                *occ.get_mut() += 1;
            }
            Entry::Vacant(vac) => {
                vac.insert_entry(1);
            }
        }

        if city_lat == 0.0 && city_lon == 0.0 {
            return;
        }

        let key = VisitorLogKey {
            latitude_bytes,
            longitude_bytes,
            ip_address: ip,
            city: ip_info.city,
            country: ip_info.country_name,
        };

        let mut lock = self.visitor_log_buffer.lock().await;
        match lock.get_mut(&key) {
            Some(batch) => {
                batch.count = batch.count.saturating_add(1);
                batch.visited_at = chrono::Utc::now();
            }
            None => {
                lock.insert(
                    key,
                    VisitorLogBatch {
                        count: 1,
                        visited_at: chrono::Utc::now(),
                    },
                );
            }
        }
    }

    pub async fn flush_visitor_logs(&self) -> anyhow::Result<u64> {
        let pending = {
            let mut lock = self.visitor_log_buffer.lock().await;
            if lock.is_empty() {
                return Ok(0);
            }
            std::mem::take(&mut *lock)
        };

        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                self.requeue_visitor_logs(pending).await;
                return Err(e);
            }
        };

        let mut rows: Vec<NewVisitationData> = Vec::new();
        let mut total_pending = 0u64;
        for (key, batch) in &pending {
            let latitude = f64::from_be_bytes(key.latitude_bytes);
            let longitude = f64::from_be_bytes(key.longitude_bytes);
            if latitude.is_nan() || longitude.is_nan() {
                continue;
            }

            for _ in 0..batch.count {
                rows.push(NewVisitationData {
                    latitude,
                    longitude,
                    ip_address: ipnet::IpNet::from(key.ip_address),
                    city: key.city.clone(),
                    country: key.country.clone(),
                    visited_at: batch.visited_at,
                });
                total_pending = total_pending.saturating_add(1);
            }
        }

        if rows.is_empty() {
            return Ok(0);
        }

        let insert_result = diesel::insert_into(crate::schema::visitation_data::table)
            .values(&rows)
            .execute(&mut conn)
            .await;

        match insert_result {
            Ok(inserted_rows) => {
                info!(
                    rows_flushed = inserted_rows,
                    visit_count = total_pending,
                    "Flushed buffered visitor logs"
                );
                Ok(total_pending)
            }
            Err(e) => {
                self.requeue_visitor_logs(pending).await;
                Err(e.into())
            }
        }
    }

    async fn requeue_visitor_logs(&self, pending: StdHashMap<VisitorLogKey, VisitorLogBatch>) {
        let mut lock = self.visitor_log_buffer.lock().await;
        for (key, batch) in pending {
            match lock.get_mut(&key) {
                Some(existing) => {
                    existing.count = existing.count.saturating_add(batch.count);
                    if batch.visited_at > existing.visited_at {
                        existing.visited_at = batch.visited_at;
                    }
                }
                None => {
                    lock.insert(key, batch);
                }
            }
        }
    }

    pub async fn get_visitor_board_entries(&self) -> Vec<((f64, f64), u64)> {
        let mut result = Vec::new();
        self.visitor_board_map
            .iter_async(|&(lat_bytes, long_bytes), &count| {
                let lat = f64::from_be_bytes(lat_bytes);
                let long = f64::from_be_bytes(long_bytes);
                if !lat.is_nan() && !long.is_nan() {
                    result.push(((lat, long), count));
                }
                true
            })
            .await;
        result
    }
}
