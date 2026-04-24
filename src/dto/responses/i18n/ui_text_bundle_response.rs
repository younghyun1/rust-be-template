use std::collections::HashMap;

use serde_derive::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct UiTextBundleResponse {
    pub locale: String,
    pub fallback_locale: String,
    pub texts: HashMap<String, String>,
}
