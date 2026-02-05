use chrono::{DateTime, Utc};
use serde_derive::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::wasm_module::wasm_module::WasmModule;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WasmModuleItem {
    pub wasm_module_id: Uuid,
    pub user_id: Uuid,
    pub wasm_module_title: String,
    pub wasm_module_description: String,
    pub wasm_module_link: String,
    pub wasm_module_thumbnail_link: String,
    pub wasm_module_created_at: DateTime<Utc>,
    pub wasm_module_updated_at: DateTime<Utc>,
}

impl From<WasmModule> for WasmModuleItem {
    fn from(m: WasmModule) -> Self {
        Self {
            wasm_module_id: m.wasm_module_id,
            user_id: m.user_id,
            wasm_module_title: m.wasm_module_title,
            wasm_module_description: m.wasm_module_description,
            wasm_module_link: m.wasm_module_link,
            wasm_module_thumbnail_link: m.wasm_module_thumbnail_link,
            wasm_module_created_at: m.wasm_module_created_at,
            wasm_module_updated_at: m.wasm_module_updated_at,
        }
    }
}
