use crate::domain::i18n::i18n::InternationalizationString;
use bitcode::encode;
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use super::i18n::InternationalizationStringsToBeEncoded;

type CountryLanguageKey = (i32, i32);

pub struct I18nCache {
    pub rows: Vec<InternationalizationString>,
    // HashMap indexes
    pub country_idx: HashMap<i32, Vec<usize>>,
    pub subdivision_idx: HashMap<Option<String>, Vec<usize>>,
    pub language_idx: HashMap<i32, Vec<usize>>,
    pub created_by_idx: HashMap<Uuid, Vec<usize>>,
    pub updated_by_idx: HashMap<Uuid, Vec<usize>>,
    pub reference_idx: HashMap<String, Vec<usize>>,
    // BTreeMap indexes
    pub created_at_idx: BTreeMap<DateTime<Utc>, Vec<usize>>,
    pub updated_at_idx: BTreeMap<DateTime<Utc>, Vec<usize>>,
    pub bundle_cache: HashMap<CountryLanguageKey, (DateTime<Utc>, Vec<u8>)>,
}

impl Default for I18nCache {
    fn default() -> Self {
        Self::new()
    }
}

impl I18nCache {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            country_idx: HashMap::new(),
            subdivision_idx: HashMap::new(),
            language_idx: HashMap::new(),
            created_by_idx: HashMap::new(),
            updated_by_idx: HashMap::new(),
            reference_idx: HashMap::new(),
            created_at_idx: BTreeMap::new(),
            updated_at_idx: BTreeMap::new(),
            bundle_cache: HashMap::new(),
        }
    }

    pub fn from_rows(rows: Vec<InternationalizationString>) -> Self {
        let mut cache = I18nCache::new();
        for (i, row) in rows.into_iter().enumerate() {
            // Push to row vec, retain `i` as index
            cache.rows.push(row);
            let row_ref = &cache.rows[i];
            // country
            cache
                .country_idx
                .entry(row_ref.i18n_string_country_code)
                .or_default()
                .push(i);
            // subdivision
            cache
                .subdivision_idx
                .entry(row_ref.i18n_string_country_subdivision_code.clone())
                .or_default()
                .push(i);
            // language
            cache
                .language_idx
                .entry(row_ref.i18n_string_language_code)
                .or_default()
                .push(i);
            // created_by
            cache
                .created_by_idx
                .entry(row_ref.i18n_string_created_by)
                .or_default()
                .push(i);
            // updated_by
            cache
                .updated_by_idx
                .entry(row_ref.i18n_string_updated_by)
                .or_default()
                .push(i);
            // reference_key
            cache
                .reference_idx
                .entry(row_ref.i18n_string_reference_key.clone())
                .or_default()
                .push(i);
            // created_at
            cache
                .created_at_idx
                .entry(row_ref.i18n_string_created_at)
                .or_default()
                .push(i);
            // updated_at
            cache
                .updated_at_idx
                .entry(row_ref.i18n_string_updated_at)
                .or_default()
                .push(i);
        }
        cache
    }

    // Example lookup methods:
    pub fn by_country(&self, code: i32) -> Vec<&InternationalizationString> {
        self.country_idx
            .get(&code)
            .map(|v| v.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }
    pub fn by_subdivision(&self, code: Option<&str>) -> Vec<&InternationalizationString> {
        let key = code.map(|s| s.to_string());
        self.subdivision_idx
            .get(&key)
            .map(|v| v.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }
    pub fn by_language(&self, code: i32) -> Vec<&InternationalizationString> {
        self.language_idx
            .get(&code)
            .map(|v| v.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }
    pub fn by_reference(&self, key: &str) -> Vec<&InternationalizationString> {
        self.reference_idx
            .get(key)
            .map(|v| v.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }
    pub fn by_created_by(&self, user: &Uuid) -> Vec<&InternationalizationString> {
        self.created_by_idx
            .get(user)
            .map(|v| v.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }
    pub fn by_updated_by(&self, user: &Uuid) -> Vec<&InternationalizationString> {
        self.updated_by_idx
            .get(user)
            .map(|v| v.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }

    pub fn range_created_at(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&InternationalizationString> {
        self.created_at_idx
            .range(start..=end)
            .flat_map(|(_k, v)| v.iter().map(|&i| &self.rows[i]))
            .collect()
    }

    pub fn range_updated_at(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&InternationalizationString> {
        self.updated_at_idx
            .range(start..=end)
            .flat_map(|(_k, v)| v.iter().map(|&i| &self.rows[i]))
            .collect()
    }

    pub fn latest_updated_at_for_country_language(
        &self,
        country_code: i32,
        language_code: i32,
    ) -> Option<DateTime<Utc>> {
        for (_, indices) in self.updated_at_idx.iter().rev() {
            for &idx in indices.iter().rev() {
                let row = &self.rows[idx];
                if row.i18n_string_country_code == country_code
                    && row.i18n_string_language_code == language_code
                {
                    return Some(row.i18n_string_updated_at);
                }
            }
        }
        None
    }

    pub fn build_country_language_bundle(
        &self,
        country_code: i32,
        language_code: i32,
    ) -> (Vec<u8>, Option<DateTime<Utc>>) {
        let rows: Vec<_> = self
            .rows
            .iter()
            .filter(|row| {
                row.i18n_string_country_code == country_code
                    && row.i18n_string_language_code == language_code
            })
            .cloned()
            .collect();

        if rows.is_empty() {
            return (vec![], None);
        }

        let max_updated_at = rows
            .iter()
            .map(|row| row.i18n_string_updated_at)
            .max()
            .unwrap();

        let to_encode: Vec<InternationalizationStringsToBeEncoded> = rows
            .into_iter()
            .map(InternationalizationStringsToBeEncoded::from)
            .collect();

        (encode(&to_encode), Some(max_updated_at))
    }
}
