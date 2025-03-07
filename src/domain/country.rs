use std::collections::{BTreeMap, HashMap};

use diesel::{Queryable, QueryableByName};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use serde_derive::{Deserialize, Serialize};

use crate::schema::iso_country;

#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = iso_country)]
pub struct IsoCountry {
    #[diesel(sql_type = diesel::sql_types::Integer, column_name = country_code)]
    pub country_code: i32,
    #[diesel(sql_type = diesel::sql_types::VarChar, column_name = country_alpha2)]
    pub country_alpha2: String,
    #[diesel(sql_type = diesel::sql_types::VarChar, column_name = country_alpha3)]
    pub country_alpha3: String,
    #[diesel(sql_type = diesel::sql_types::VarChar, column_name = country_eng_name)]
    pub country_eng_name: String,
    #[diesel(sql_type = diesel::sql_types::Integer, column_name = country_currency)]
    pub country_currency: i32,
    #[diesel(sql_type = diesel::sql_types::VarChar, column_name = phone_prefix)]
    pub phone_prefix: String,
    #[diesel(sql_type = diesel::sql_types::VarChar, column_name = country_flag)]
    pub country_flag: String,
    #[diesel(sql_type = diesel::sql_types::Bool, column_name = is_country)]
    pub is_country: bool,
    #[diesel(sql_type = diesel::sql_types::Integer, column_name = country_primary_language)]
    pub country_primary_language: i32,
}

#[derive(Serialize)]
pub struct IsoCountryTable {
    rows: Vec<IsoCountry>,
    alpha2_index: HashMap<String, usize>,
    alpha3_index: HashMap<String, usize>,
    serialized_list: serde_json::Value,
}

impl From<Vec<IsoCountry>> for IsoCountryTable {
    fn from(mut rows: Vec<IsoCountry>) -> Self {
        // Sort the rows by the English country name so that they appear in order.
        rows.sort_by_key(|row| row.country_eng_name.clone());

        let mut alpha2_index = HashMap::new();
        let mut alpha3_index = HashMap::new();
        for (idx, row) in rows.iter().enumerate() {
            alpha2_index.insert(row.country_alpha2.clone(), idx);
            alpha3_index.insert(row.country_alpha3.clone(), idx);
        }

        let serialized_list = serde_json::json!({ "countries": rows });

        IsoCountryTable {
            rows,
            alpha2_index,
            alpha3_index,
            serialized_list,
        }
    }
}

impl IsoCountryTable {
    pub fn lookup_by_alpha2(&self, code: &str) -> Option<IsoCountry> {
        self.alpha2_index
            .get(code)
            .map(|&idx| self.rows[idx].clone())
    }

    pub fn lookup_by_alpha3(&self, code: &str) -> Option<IsoCountry> {
        self.alpha3_index
            .get(code)
            .map(|&idx| self.rows[idx].clone())
    }

    pub async fn load_from_db<'conn>(
        mut conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        let countries: Vec<IsoCountry> = iso_country::table.load(&mut conn).await?;
        Ok(countries.into())
    }

    pub fn new_empty() -> Self {
        IsoCountryTable {
            rows: Vec::new(),
            alpha2_index: HashMap::new(),
            alpha3_index: HashMap::new(),
            serialized_list: serde_json::Value::Null,
        }
    }
}

// 1. ISO Country Subdivision
#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = iso_country_subdivision)]
pub struct IsoCountrySubdivision {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub subdivision_id: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub country_code: i32,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub subdivision_code: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub subdivision_name: String,
    // Note: subdivision_type is nullable in the DB so we make it Option.
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub subdivision_type: Option<String>,
}

// Create an “indexed table‐like” struct for subdivisions.
#[derive(Serialize)]
pub struct IsoCountrySubdivisionTable {
    // The raw rows.
    pub rows: Vec<IsoCountrySubdivision>,
    // Index by primary key.
    pub by_id: HashMap<i32, usize>,
    // An index mapping (country_code, subdivision_code) to index.
    /// (Note: You might decide to index by country-code string or by some other key,
    ///  in which case adjust the key type as needed.)
    pub by_country_and_code: HashMap<(i32, String), usize>,
}

impl From<Vec<IsoCountrySubdivision>> for IsoCountrySubdivisionTable {
    fn from(rows: Vec<IsoCountrySubdivision>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_country_and_code = HashMap::new();
        for (idx, row) in rows.iter().enumerate() {
            by_id.insert(row.subdivision_id, idx);
            by_country_and_code.insert((row.country_code, row.subdivision_code.clone()), idx);
        }
        IsoCountrySubdivisionTable {
            rows,
            by_id,
            by_country_and_code,
        }
    }
}

impl IsoCountrySubdivisionTable {
    pub fn lookup_by_subdivision_id(&self, id: i32) -> Option<IsoCountrySubdivision> {
        self.by_id.get(&id).map(|&idx| self.rows[idx].clone())
    }

    pub fn lookup_by_country_and_code(
        &self,
        country_code: i32,
        code: &str,
    ) -> Option<IsoCountrySubdivision> {
        self.by_country_and_code
            .get(&(country_code, code.to_owned()))
            .map(|&idx| self.rows[idx].clone())
    }

    pub fn new_empty() -> Self {
        IsoCountrySubdivisionTable {
            rows: Vec::new(),
            by_id: HashMap::new(),
            by_country_and_code: HashMap::new(),
        }
    }
}

// 2. ISO Currency
#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = iso_currency)]
pub struct IsoCurrency {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub currency_code: i32,
    // Note: for bpchar(3) we can still use String.
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub currency_alpha3: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub currency_name: String,
}

// Create an indexed currency table.
#[derive(Serialize)]
pub struct IsoCurrencyTable {
    pub rows: Vec<IsoCurrency>,
    pub by_code: HashMap<i32, usize>,
    pub by_alpha3: HashMap<String, usize>,
}

impl From<Vec<IsoCurrency>> for IsoCurrencyTable {
    fn from(rows: Vec<IsoCurrency>) -> Self {
        let mut by_code = HashMap::new();
        let mut by_alpha3 = HashMap::new();
        for (idx, row) in rows.iter().enumerate() {
            by_code.insert(row.currency_code, idx);
            by_alpha3.insert(row.currency_alpha3.clone(), idx);
        }
        IsoCurrencyTable {
            rows,
            by_code,
            by_alpha3,
        }
    }
}

impl IsoCurrencyTable {
    pub fn lookup_by_code(&self, code: i32) -> Option<IsoCurrency> {
        self.by_code.get(&code).map(|&idx| self.rows[idx].clone())
    }

    pub fn lookup_by_alpha3(&self, alpha3: &str) -> Option<IsoCurrency> {
        self.by_alpha3
            .get(alpha3)
            .map(|&idx| self.rows[idx].clone())
    }

    pub fn new_empty() -> Self {
        IsoCurrencyTable {
            rows: Vec::new(),
            by_code: HashMap::new(),
            by_alpha3: HashMap::new(),
        }
    }
}

// 3. ISO Language
#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = iso_language)]
pub struct IsoLanguage {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub language_code: i32,
    // bpchar(2) means a fixed-length char string. We use String here.
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub language_alpha2: String,
    // bpchar(3) for alpha3.
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub language_alpha3: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub language_eng_name: String,
}

// Create an indexed language table.
#[derive(Serialize)]
pub struct IsoLanguageTable {
    pub rows: Vec<IsoLanguage>,
    pub by_code: HashMap<i32, usize>,
    pub by_alpha2: HashMap<String, usize>,
    pub by_alpha3: HashMap<String, usize>,
    pub serialized_map: serde_json::Value,
}

impl From<Vec<IsoLanguage>> for IsoLanguageTable {
    fn from(rows: Vec<IsoLanguage>) -> Self {
        let mut by_code = HashMap::new();
        let mut by_alpha2 = HashMap::new();
        let mut by_alpha3 = HashMap::new();
        for (idx, row) in rows.iter().enumerate() {
            by_code.insert(row.language_code, idx);
            by_alpha2.insert(row.language_alpha2.clone(), idx);
            by_alpha3.insert(row.language_alpha3.clone(), idx);
        }

        let mut languages: BTreeMap<i32, TruncatedLanguage> = BTreeMap::new();

        rows.iter().for_each(|row| {
            languages.insert(
                row.language_code,
                TruncatedLanguage {
                    language_alpha2: row.language_alpha2.clone(),
                    language_alpha3: row.language_alpha3.clone(),
                    language_eng_name: row.language_eng_name.clone(),
                },
            );
        });

        let serialized_map: serde_json::Value = serde_json::to_value(&languages).unwrap();

        IsoLanguageTable {
            rows,
            by_code,
            by_alpha2,
            by_alpha3,
            serialized_map,
        }
    }
}

impl IsoLanguageTable {
    pub fn lookup_by_code(&self, code: i32) -> Option<IsoLanguage> {
        self.by_code.get(&code).map(|&idx| self.rows[idx].clone())
    }

    pub fn lookup_by_alpha2(&self, alpha2: &str) -> Option<IsoLanguage> {
        self.by_alpha2
            .get(alpha2)
            .map(|&idx| self.rows[idx].clone())
    }

    pub fn lookup_by_alpha3(&self, alpha3: &str) -> Option<IsoLanguage> {
        self.by_alpha3
            .get(alpha3)
            .map(|&idx| self.rows[idx].clone())
    }

    pub fn new_empty() -> Self {
        IsoLanguageTable {
            rows: Vec::new(),
            by_code: HashMap::new(),
            by_alpha2: HashMap::new(),
            by_alpha3: HashMap::new(),
            serialized_map: serde_json::Value::Null,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CountryAndSubdivisions {
    pub country: IsoCountry,
    // All subdivisions belonging to that country.
    pub subdivisions: Vec<IsoCountrySubdivision>,
}

// A table‐like wrapper that holds many combined records, plus indexes.
// An extra JSON snapshot is included to be dispatched directly.
#[derive(Serialize)]
pub struct CountryAndSubdivisionsTable {
    /// Combined records keyed by country.
    pub rows: Vec<CountryAndSubdivisions>,
    pub by_id: HashMap<i32, usize>,
    /// An index from a country's alpha‑2 code to its combined record.
    pub by_country_alpha2: HashMap<String, usize>,
    /// An index from a country's alpha‑3 code to its combined record.
    pub by_country_alpha3: HashMap<String, usize>,
    /// A JSON representation of the combined table ready for dispatch.
    pub serialized_country_list: serde_json::Value,
}

impl CountryAndSubdivisionsTable {
    /// Build the combined table given vectors of IsoCountry records and subdivisions.
    pub fn new(countries: Vec<IsoCountry>, subdivisions: Vec<IsoCountrySubdivision>) -> Self {
        // First, build a temporary lookup for countries keyed by country_code.
        let mut country_map: HashMap<i32, IsoCountry> = HashMap::new();
        for country in countries {
            country_map.insert(country.country_code, country);
        }

        // Then, create a map from country_code to its subdivisions.
        let mut subdivisions_map: HashMap<i32, Vec<IsoCountrySubdivision>> = HashMap::new();
        for subdiv in subdivisions {
            subdivisions_map
                .entry(subdiv.country_code)
                .or_insert_with(Vec::new)
                .push(subdiv);
        }

        // Combine the data, even if there are no subdivisions for a given country.
        let mut rows: Vec<CountryAndSubdivisions> = country_map
            .into_iter()
            .map(|(country_code, country)| {
                let subs = subdivisions_map.remove(&country_code).unwrap_or_default();
                CountryAndSubdivisions {
                    country,
                    subdivisions: subs,
                }
            })
            .collect();

        // Sort the combined records by the country's English name.
        rows.sort_by_key(|combined| combined.country.country_eng_name.clone());

        // Build the indexes.
        let mut by_id = HashMap::new();
        let mut by_country_alpha2 = HashMap::new();
        let mut by_country_alpha3 = HashMap::new();
        for (idx, combined) in rows.iter().enumerate() {
            by_id.insert(combined.country.country_code.clone(), idx);
            by_country_alpha2.insert(combined.country.country_alpha2.clone(), idx);
            by_country_alpha3.insert(combined.country.country_alpha3.clone(), idx);
        }

        // Build a JSON representation ready to be dispatched, excluding subdivisions.
        let dispatch_json = serde_json::json!({
            "countries": rows.iter().map(|combined| &combined.country).collect::<Vec<_>>()
        });

        CountryAndSubdivisionsTable {
            rows,
            by_id,
            by_country_alpha2,
            by_country_alpha3,
            serialized_country_list: dispatch_json,
        }
    }

    /// Create an empty table.
    pub fn new_empty() -> Self {
        CountryAndSubdivisionsTable {
            rows: Vec::new(),
            by_id: HashMap::new(),
            by_country_alpha2: HashMap::new(),
            by_country_alpha3: HashMap::new(),
            serialized_country_list: serde_json::json!({ "countries": [] }),
        }
    }

    /// Lookup a combined record by its country's alpha-2 code.
    pub fn lookup_by_alpha2(&self, code: &str) -> Option<&CountryAndSubdivisions> {
        self.by_country_alpha2.get(code).map(|&idx| &self.rows[idx])
    }

    /// Lookup a combined record by its country's alpha-3 code.
    pub fn lookup_by_alpha3(&self, code: &str) -> Option<&CountryAndSubdivisions> {
        self.by_country_alpha3.get(code).map(|&idx| &self.rows[idx])
    }

    /// Optionally retrieve the JSON representation on demand.
    pub fn as_dispatch_json(&self) -> serde_json::Value {
        serde_json::json!({ "countries": self.rows })
    }
}

#[derive(Serialize)]
pub struct TruncatedLanguage {
    pub language_alpha2: String,
    pub language_alpha3: String,
    pub language_eng_name: String,
}
