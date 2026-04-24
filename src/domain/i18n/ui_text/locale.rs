use serde_derive::Serialize;
use utoipa::ToSchema;

pub const EN_US_COUNTRY_CODE: i32 = 840;
pub const EN_US_LANGUAGE_CODE: i32 = 41;
pub const KO_KR_COUNTRY_CODE: i32 = 410;
pub const KO_KR_LANGUAGE_CODE: i32 = 86;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ToSchema)]
pub enum UiLocale {
    EnUs,
    KoKr,
}

impl UiLocale {
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("ko") | Some("ko-KR") | Some("ko_kr") | Some("ko-kr") => UiLocale::KoKr,
            Some("en") | Some("en-US") | Some("en_us") | Some("en-us") => UiLocale::EnUs,
            _ => UiLocale::EnUs,
        }
    }

    pub fn as_tag(self) -> &'static str {
        match self {
            UiLocale::EnUs => "en-US",
            UiLocale::KoKr => "ko-KR",
        }
    }

    pub fn language_code(self) -> i32 {
        match self {
            UiLocale::EnUs => EN_US_LANGUAGE_CODE,
            UiLocale::KoKr => KO_KR_LANGUAGE_CODE,
        }
    }

    pub fn country_code(self) -> i32 {
        match self {
            UiLocale::EnUs => EN_US_COUNTRY_CODE,
            UiLocale::KoKr => KO_KR_COUNTRY_CODE,
        }
    }
}
