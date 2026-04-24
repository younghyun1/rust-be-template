use serde_derive::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetUiTextBundleRequest {
    pub locale: Option<String>,
}
