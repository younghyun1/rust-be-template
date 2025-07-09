use crate::schema::visitation_data;
use diesel::{
    Selectable,
    prelude::{Queryable, QueryableByName},
};

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = visitation_data)]
pub struct VisitationData {
    pub visitation_data_id: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub ip_address: std::net::IpAddr,
    pub city: String,
    pub country: String,
    pub visited_at: chrono::DateTime<chrono::Utc>,
}
