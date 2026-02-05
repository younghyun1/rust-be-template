use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Insertable, Queryable, QueryableByName, Selectable};
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::schema::wasm_module;

#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable, Selectable, ToSchema)]
#[diesel(table_name = wasm_module)]
pub struct WasmModule {
    pub wasm_module_id: Uuid,
    pub user_id: Uuid,
    pub wasm_module_link: String,
    pub wasm_module_description: String,
    pub wasm_module_created_at: DateTime<Utc>,
    pub wasm_module_updated_at: DateTime<Utc>,
    pub wasm_module_thumbnail_link: String,
    pub wasm_module_title: String,
}

#[derive(Insertable)]
#[diesel(table_name = wasm_module)]
pub struct WasmModuleInsertable {
    pub wasm_module_id: Uuid,
    pub user_id: Uuid,
    pub wasm_module_link: String,
    pub wasm_module_description: String,
    pub wasm_module_created_at: DateTime<Utc>,
    pub wasm_module_updated_at: DateTime<Utc>,
    pub wasm_module_thumbnail_link: String,
    pub wasm_module_title: String,
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = wasm_module)]
pub struct WasmModuleChangeset {
    pub wasm_module_title: Option<String>,
    pub wasm_module_description: Option<String>,
    pub wasm_module_updated_at: Option<DateTime<Utc>>,
}
