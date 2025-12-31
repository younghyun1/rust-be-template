use chrono::{DateTime, Utc};
use diesel::prelude::{Insertable, Queryable, QueryableByName};
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::schema::photographs;

#[derive(Serialize, Deserialize, QueryableByName, Queryable, ToSchema)]
#[diesel(table_name = photographs)]
pub struct Photograph {
    pub photograph_id: Uuid,
    pub user_id: Uuid,
    pub photograph_shot_at: Option<DateTime<Utc>>,
    pub photograph_created_at: DateTime<Utc>,
    pub photograph_updated_at: DateTime<Utc>,
    pub photograph_image_type: i32,
    pub photograph_is_on_cloud: bool,
    pub photograph_link: String,
    pub photograph_comments: String,
    pub photograph_lat: f64,
    pub photograph_lon: f64,
    pub photograph_thumbnail_link: String,
}

#[derive(Insertable)]
#[diesel(table_name = photographs)]
pub struct PhotographInsertable {
    pub user_id: Uuid,
    pub photograph_shot_at: Option<DateTime<Utc>>,
    pub photograph_image_type: i32,
    pub photograph_is_on_cloud: bool,
    pub photograph_link: String,
    pub photograph_comments: String,
    pub photograph_lat: f64,
    pub photograph_lon: f64,
    pub photograph_thumbnail_link: String,
}
