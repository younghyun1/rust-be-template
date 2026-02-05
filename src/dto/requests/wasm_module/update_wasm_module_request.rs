use serde_derive::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWasmModuleRequest {
    pub wasm_module_title: Option<String>,
    pub wasm_module_description: Option<String>,
}
