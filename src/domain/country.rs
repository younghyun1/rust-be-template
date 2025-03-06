use std::collections::HashMap;

use diesel::{Queryable, QueryableByName};
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
pub struct IsoCountry {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub country_code: i32,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub country_alpha2: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub country_alpha3: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub country_eng_name: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub country_primary_language: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub country_currency: i32,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub phone_prefix: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub country_flag: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_country: bool,
}

#[derive(Serialize)]
pub struct IsoCountryTable {
    rows: Vec<IsoCountry>,
    alpha2_index: HashMap<String, usize>,
    alpha3_index: HashMap<String, usize>,
}

impl From<Vec<IsoCountry>> for IsoCountryTable {
    fn from(rows: Vec<IsoCountry>) -> Self {
        let mut alpha2_index = HashMap::new();
        let mut alpha3_index = HashMap::new();
        for (idx, row) in rows.iter().enumerate() {
            alpha2_index.insert(row.country_alpha2.clone(), idx);
            alpha3_index.insert(row.country_alpha3.clone(), idx);
        }
        IsoCountryTable {
            rows,
            alpha2_index,
            alpha3_index,
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
}

// 1. ISO Country Subdivision
#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
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
}

// 2. ISO Currency
#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
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
}

// 3. ISO Language
#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable)]
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
        IsoLanguageTable {
            rows,
            by_code,
            by_alpha2,
            by_alpha3,
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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CountryAndSubdivisions {
    pub country: IsoCountry,
    // All subdivisions belonging to that country.
    pub subdivisions: Vec<IsoCountrySubdivision>,
}

// A table-like wrapper that holds many combined records, plus indexes.
#[derive(Serialize)]
pub struct CountryAndSubdivisionsTable {
    /// Combined records keyed by country.
    pub rows: Vec<CountryAndSubdivisions>,
    /// An index from a country's alpha-2 code to its combined record.
    pub by_country_alpha2: HashMap<String, usize>,
    /// An index from a country's alpha-3 code to its combined record.
    pub by_country_alpha3: HashMap<String, usize>,
}

impl CountryAndSubdivisionsTable {
    /// Build the combined table given a vector of IsoCountry records and a vector of subdivisions.
    pub fn new(countries: Vec<IsoCountry>, subdivisions: Vec<IsoCountrySubdivision>) -> Self {
        // First, build a temporary lookup for countries by country_code.
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

        // Now, combine the data, even if there are no subdivisions.
        let mut rows = Vec::new();
        for (country_code, country) in country_map.into_iter() {
            let subs = subdivisions_map.remove(&country_code).unwrap_or_default();
            rows.push(CountryAndSubdivisions {
                country,
                subdivisions: subs,
            });
        }

        // Build the indexes.
        let mut by_country_alpha2 = HashMap::new();
        let mut by_country_alpha3 = HashMap::new();
        for (idx, combined) in rows.iter().enumerate() {
            by_country_alpha2.insert(combined.country.country_alpha2.clone(), idx);
            by_country_alpha3.insert(combined.country.country_alpha3.clone(), idx);
        }

        CountryAndSubdivisionsTable {
            rows,
            by_country_alpha2,
            by_country_alpha3,
        }
    }

    // Lookup by country's alpha2 code.
    pub fn lookup_by_country_alpha2(&self, code: &str) -> Option<&CountryAndSubdivisions> {
        self.by_country_alpha2.get(code).map(|&idx| &self.rows[idx])
    }

    // Lookup by country's alpha3 code.
    pub fn lookup_by_country_alpha3(&self, code: &str) -> Option<&CountryAndSubdivisions> {
        self.by_country_alpha3.get(code).map(|&idx| &self.rows[idx])
    }

    // Additionally, if you need to search for a subdivision across all countries by its subdivision ID:
    pub fn lookup_subdivision_by_id(
        &self,
        subdivision_id: i32,
    ) -> Option<(&IsoCountry, &IsoCountrySubdivision)> {
        for combined in &self.rows {
            if let Some(subdiv) = combined
                .subdivisions
                .iter()
                .find(|s| s.subdivision_id == subdivision_id)
            {
                return Some((&combined.country, subdiv));
            }
        }
        None
    }
}
