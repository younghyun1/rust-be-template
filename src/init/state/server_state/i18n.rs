use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::info;
use uuid::Uuid;

use super::ServerState;
use crate::domain::country::{
    CountryAndSubdivisionsTable, IsoCountry, IsoCountrySubdivision, IsoCurrency, IsoCurrencyTable,
    IsoLanguage, IsoLanguageTable,
};
use crate::domain::i18n::i18n::InternationalizationString;
use crate::domain::i18n::i18n_cache::I18nCache;
use crate::domain::i18n::ui_text::source::source_bundles;
use crate::schema::{
    i18n_strings, iso_country, iso_country_subdivision, iso_currency, iso_language,
};
use crate::util::time::now::tokio_now;

impl ServerState {
    pub async fn sync_country_data(&self) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();

        let country_fut = async {
            let mut conn = self.get_conn().await?;
            let countries: Vec<IsoCountry> = iso_country::table.load(&mut conn).await?;
            let subdivisions: Vec<IsoCountrySubdivision> =
                iso_country_subdivision::table.load(&mut conn).await?;
            let total_rows = countries.len() + subdivisions.len();
            Ok::<(CountryAndSubdivisionsTable, usize), anyhow::Error>((
                CountryAndSubdivisionsTable::new(countries, subdivisions),
                total_rows,
            ))
        };

        let language_fut = async {
            let mut conn = self.get_conn().await?;
            let languages: Vec<IsoLanguage> = iso_language::table.load(&mut conn).await?;
            Ok::<(IsoLanguageTable, usize), anyhow::Error>((
                IsoLanguageTable::from(languages.clone()),
                languages.len(),
            ))
        };

        let currency_fut = async {
            let mut conn = self.get_conn().await?;
            let currencies: Vec<IsoCurrency> = iso_currency::table.load(&mut conn).await?;
            Ok::<(IsoCurrencyTable, usize), anyhow::Error>((
                IsoCurrencyTable::from(currencies.clone()),
                currencies.len(),
            ))
        };

        let (country_res, lang_res, curr_res) =
            tokio::join!(country_fut, language_fut, currency_fut);

        if let Ok((new_country_map, country_rows)) = country_res {
            let mut lock = self.country_map.write().await;
            *lock = new_country_map;
            info!(rows_synchronized = %country_rows, "Synchronized country data data.");
        } else if let Err(e) = country_res {
            tracing::error!(error = ?e, "Error synchronizing country data");
        }

        if let Ok((new_langs_map, lang_rows)) = lang_res {
            let mut lock = self.languages_map.write().await;
            *lock = new_langs_map;
            info!(rows_synchronized = %lang_rows, "Synchronized language data.");
        } else if let Err(e) = lang_res {
            tracing::error!(error = ?e, "Error synchronizing languages data");
        }

        if let Ok((new_currency_map, curr_rows)) = curr_res {
            let mut lock = self.currency_map.write().await;
            *lock = new_currency_map;
            info!(rows_synchronized = %curr_rows, "Synchronized currency data.");
        } else if let Err(e) = curr_res {
            tracing::error!(error = ?e, "Error synchronizing currency data");
        }

        let elapsed = start.elapsed();
        info!(elapsed = %format!("{:?}", elapsed), "Country/language/currency data cache synchronized.");

        Ok(())
    }

    pub async fn sync_i18n_data(&self) -> anyhow::Result<usize> {
        let start = tokio_now();

        let rows = InternationalizationString::get_all(self.get_conn().await?).await?;
        let num_rows = rows.len();
        let mut lock = self.i18n_cache.write().await;
        *lock = I18nCache::from_rows(rows);

        info!(elapsed = ?start.elapsed(), rows_synchronized = %num_rows, "Synchronized i18n data.");
        Ok(num_rows)
    }

    pub async fn sync_file_backed_ui_text_sources(&self) -> anyhow::Result<usize> {
        let start = tokio_now();
        let bundles = source_bundles()?;
        let mut conn = self.get_conn().await?;
        let system_user_id = Uuid::nil();
        let mut rows_synchronized = 0usize;

        for bundle in bundles {
            for entry in bundle.entries {
                let now = chrono::Utc::now();
                let updated_rows = diesel::update(
                    i18n_strings::table
                        .filter(i18n_strings::i18n_string_reference_key.eq(&entry.key))
                        .filter(
                            i18n_strings::i18n_string_country_code.eq(bundle.locale.country_code()),
                        )
                        .filter(
                            i18n_strings::i18n_string_language_code
                                .eq(bundle.locale.language_code()),
                        )
                        .filter(i18n_strings::i18n_string_country_subdivision_code.is_null()),
                )
                .set((
                    i18n_strings::i18n_string_content.eq(&entry.content),
                    i18n_strings::i18n_string_updated_at.eq(now),
                    i18n_strings::i18n_string_updated_by.eq(system_user_id),
                ))
                .execute(&mut conn)
                .await?;

                if updated_rows == 0 {
                    diesel::insert_into(i18n_strings::table)
                        .values((
                            i18n_strings::i18n_string_content.eq(&entry.content),
                            i18n_strings::i18n_string_updated_by.eq(system_user_id),
                            i18n_strings::i18n_string_language_code
                                .eq(bundle.locale.language_code()),
                            i18n_strings::i18n_string_country_code.eq(bundle.locale.country_code()),
                            i18n_strings::i18n_string_country_subdivision_code
                                .eq(Option::<String>::None),
                            i18n_strings::i18n_string_reference_key.eq(&entry.key),
                        ))
                        .execute(&mut conn)
                        .await?;
                } else if updated_rows > 1 {
                    tracing::warn!(
                        locale = %bundle.locale.as_tag(),
                        key = %entry.key,
                        duplicate_rows = %updated_rows,
                        "Updated duplicate UI i18n rows; existing duplicate rows should be consolidated"
                    );
                }

                rows_synchronized += 1;
            }
        }

        info!(
            elapsed = ?start.elapsed(),
            rows_synchronized = %rows_synchronized,
            "Synchronized file-backed UI i18n source data."
        );
        Ok(rows_synchronized)
    }
}
