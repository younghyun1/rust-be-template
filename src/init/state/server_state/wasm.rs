use std::sync::Arc;

use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::{error, info};
use uuid::Uuid;

use super::ServerState;
use crate::schema::wasm_module;
use crate::util::time::now::tokio_now;
use crate::util::wasm_bundle::sniff_content_type_from_gzip_bytes;

impl ServerState {
    pub async fn sync_wasm_module_cache(&self) -> anyhow::Result<usize> {
        let start = tokio_now();
        let mut conn = self.get_conn().await?;

        let rows: Vec<(Uuid, Vec<u8>)> = wasm_module::table
            .select((
                wasm_module::wasm_module_id,
                wasm_module::wasm_module_bundle_gz,
            ))
            .load(&mut conn)
            .await?;

        drop(conn);

        let mut cached = 0usize;
        for (wasm_module_id, gz_bytes) in rows {
            if self
                .cache_wasm_module_from_gzip(wasm_module_id, gz_bytes)
                .await
                .is_some()
            {
                cached += 1;
            }
        }

        info!(
            elapsed = ?start.elapsed(),
            rows_synchronized = %cached,
            "Synchronized WASM module cache."
        );

        Ok(cached)
    }

    pub async fn upsert_wasm_module_cache(
        &self,
        wasm_module_id: Uuid,
        gz_bytes: Vec<u8>,
        content_type: &'static str,
    ) {
        let bytes: Arc<[u8]> = Arc::from(gz_bytes.into_boxed_slice());
        let entry = (bytes, true, content_type);
        let _ = self
            .wasm_module_cache
            .insert_async(wasm_module_id, entry)
            .await;
    }

    async fn cache_wasm_module_from_gzip(
        &self,
        wasm_module_id: Uuid,
        gz_bytes: Vec<u8>,
    ) -> Option<(Arc<[u8]>, bool, &'static str)> {
        let sniff_result = tokio::task::spawn_blocking(move || {
            let content_type = sniff_content_type_from_gzip_bytes(&gz_bytes)?;
            Ok::<(&'static str, Vec<u8>), anyhow::Error>((content_type, gz_bytes))
        })
        .await;

        let (content_type, gz_bytes) = match sniff_result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to sniff WASM bundle content type");
                return None;
            }
            Err(e) => {
                error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to join WASM bundle sniff task");
                return None;
            }
        };

        let bytes: Arc<[u8]> = Arc::from(gz_bytes.into_boxed_slice());
        let entry = (bytes.clone(), true, content_type);

        let _ = self
            .wasm_module_cache
            .insert_async(wasm_module_id, entry.clone())
            .await;

        info!(
            wasm_module_id = %wasm_module_id,
            size_bytes = bytes.len(),
            is_gzipped = true,
            content_type = content_type,
            "Loaded WASM module bundle into cache"
        );

        Some(entry)
    }

    pub async fn get_wasm_module(
        &self,
        wasm_module_id: Uuid,
    ) -> Option<(Arc<[u8]>, bool, &'static str)> {
        if let Some(entry) = self
            .wasm_module_cache
            .read_async(&wasm_module_id, |_, v| v.clone())
            .await
        {
            return Some(entry);
        }

        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = ?e, "Failed to get DB connection for WASM bundle");
                return None;
            }
        };

        let row: Option<(Uuid, Vec<u8>)> = wasm_module::table
            .select((wasm_module::wasm_module_id, wasm_module::wasm_module_bundle_gz))
            .filter(wasm_module::wasm_module_id.eq(wasm_module_id))
            .first(&mut conn)
            .await
            .optional()
            .map_err(|e| {
                error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to load WASM module from DB");
                e
            })
            .ok()?;

        drop(conn);

        let (_, gz_bytes) = row?;
        let entry = self
            .cache_wasm_module_from_gzip(wasm_module_id, gz_bytes)
            .await?;

        Some(entry)
    }

    pub async fn invalidate_wasm_module(&self, wasm_module_id: Uuid) {
        let _ = self.wasm_module_cache.remove_async(&wasm_module_id).await;
    }
}
