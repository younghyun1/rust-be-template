use chrono::{DateTime, Utc};
use serde_derive::Serialize;

#[derive(Serialize)]
pub struct ResponseMeta<T: serde::Serialize> {
    time_to_process: String,
    timestamp: DateTime<Utc>,
    metadata: T,
}

impl<T: serde::Serialize> ResponseMeta<T> {
    pub fn get_metadata(self) -> T {
        self.metadata
    }
}

impl<T: serde::Serialize> ResponseMeta<T> {
    pub fn from(start: tokio::time::Instant, metadata: T) -> Self {
        ResponseMeta {
            time_to_process: format!("{:?}", start.elapsed()),
            timestamp: Utc::now(),
            metadata,
        }
    }
}
