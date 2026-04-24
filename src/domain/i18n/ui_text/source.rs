use std::collections::HashSet;

use serde_json::Value;

use super::keys::REQUIRED_UI_TEXT_KEYS;
use super::locale::UiLocale;

pub struct UiTextSourceBundle {
    pub locale: UiLocale,
    pub entries: Vec<UiTextSourceEntry>,
}

pub struct UiTextSourceEntry {
    pub key: String,
    pub content: String,
}

const EN_US_JSON: &str = include_str!("../../../../i18n/ui/en-US.json");
const KO_KR_JSON: &str = include_str!("../../../../i18n/ui/ko-KR.json");

pub fn source_bundles() -> anyhow::Result<Vec<UiTextSourceBundle>> {
    let en_us = parse_bundle(UiLocale::EnUs, EN_US_JSON)?;
    let ko_kr = parse_bundle(UiLocale::KoKr, KO_KR_JSON)?;
    Ok(vec![en_us, ko_kr])
}

fn parse_bundle(locale: UiLocale, raw: &str) -> anyhow::Result<UiTextSourceBundle> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("Failed to parse {} UI text JSON: {}", locale.as_tag(), e))?;

    let object = match value.as_object() {
        Some(object) => object,
        None => {
            return Err(anyhow::anyhow!(
                "{} UI text JSON must be an object",
                locale.as_tag()
            ));
        }
    };

    let mut seen = HashSet::<&str>::new();
    let mut entries = Vec::<UiTextSourceEntry>::with_capacity(object.len());

    for required_key in REQUIRED_UI_TEXT_KEYS {
        match object.get(*required_key) {
            Some(raw_content) => match raw_content.as_str() {
                Some(content) => {
                    if !seen.insert(*required_key) {
                        return Err(anyhow::anyhow!(
                            "Duplicate UI text key {} in {}",
                            required_key,
                            locale.as_tag()
                        ));
                    }
                    entries.push(UiTextSourceEntry {
                        key: (*required_key).to_string(),
                        content: content.to_string(),
                    });
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "UI text key {} in {} must be a string",
                        required_key,
                        locale.as_tag()
                    ));
                }
            },
            None => {
                return Err(anyhow::anyhow!(
                    "Missing UI text key {} in {}",
                    required_key,
                    locale.as_tag()
                ));
            }
        }
    }

    Ok(UiTextSourceBundle { locale, entries })
}
