use serde_derive::Serialize;
use utoipa::ToSchema;

use super::wasm_module_response::WasmModuleItem;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetWasmModulesResponse {
    pub items: Vec<WasmModuleItem>,
}
