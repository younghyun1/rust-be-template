use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct GetCountryLanguageBundleRequest {
    pub country_code: i32,
    pub language_code: i32,
}
