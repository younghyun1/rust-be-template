use serde_derive::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct GetCountryLanguageBundleRequest {
    pub country_code: i32,
    pub language_code: i32,
}
