use chrono::{DateTime, Utc};
use diesel::FromSqlRow;
use diesel::deserialize::{FromSql, Result as DeserializeResult};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::prelude::{Insertable, Queryable, QueryableByName};
use diesel::query_builder::QueryId;
use diesel::serialize::{IsNull, Output, ToSql};
use serde_derive::{Deserialize, Serialize};
use std::io::Write;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::schema::photographs;
use crate::schema::sql_types::PhotographContext as PhotographContextSql;

impl QueryId for PhotographContextSql {
    type QueryId = PhotographContextSql;
    const HAS_STATIC_QUERY_ID: bool = true;
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = PhotographContextSql)]
pub enum PhotographContext {
    Photography,
    Post,
}

impl PhotographContext {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "0" | "photography" | "portfolio" | "gallery" => Some(Self::Photography),
            "1" | "post" | "posts" | "blog" | "editor" => Some(Self::Post),
            _ => None,
        }
    }
}

impl ToSql<PhotographContextSql, Pg> for PhotographContext {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        let value = match self {
            PhotographContext::Photography => "photography",
            PhotographContext::Post => "post",
        };
        out.write_all(value.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<PhotographContextSql, Pg> for PhotographContext {
    fn from_sql(bytes: PgValue<'_>) -> DeserializeResult<Self> {
        match bytes.as_bytes() {
            b"photography" => Ok(PhotographContext::Photography),
            b"post" => Ok(PhotographContext::Post),
            _ => Err("Unrecognized photograph_context enum value".into()),
        }
    }
}

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
    pub photograph_context: PhotographContext,
}

#[derive(Insertable)]
#[diesel(table_name = photographs)]
pub struct PhotographInsertable {
    pub user_id: Uuid,
    pub photograph_shot_at: Option<DateTime<Utc>>,
    pub photograph_image_type: i32,
    pub photograph_context: PhotographContext,
    pub photograph_is_on_cloud: bool,
    pub photograph_link: String,
    pub photograph_comments: String,
    pub photograph_lat: f64,
    pub photograph_lon: f64,
    pub photograph_thumbnail_link: String,
}
