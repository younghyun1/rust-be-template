use std::sync::Arc;

use bitcode::Encode;
use diesel::{
    ExpressionMethods, QueryDsl,
    prelude::{Queryable, QueryableByName},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use serde_derive::{Deserialize, Serialize};

use crate::{
    errors::code_error::{CodeError, code_err},
    init::state::ServerState,
    schema::i18n_strings,
};

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
pub struct InternationalizationStrings {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub i18n_string_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub i18n_string_content: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub i18n_string_created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub i18n_string_created_by: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub i18n_string_updated_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub i18n_string_updated_by: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Int4)]
    pub i18n_string_language_code: i32,
    #[diesel(sql_type = diesel::sql_types::Int4)]
    pub i18n_string_country_code: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub i18n_string_country_subdivision_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub i18n_string_reference_key: String,
}

#[derive(Encode)]
pub struct InternationalizationStringsToBeEncoded {
    pub i18n_string_id: [u8; 16],
    pub i18n_string_content: String,
    pub i18n_string_created_at: Vec<u8>,
    pub i18n_string_created_by: [u8; 16],
    pub i18n_string_updated_at: Vec<u8>,
    pub i18n_string_updated_by: [u8; 16],
    pub i18n_string_language_code: i32,
    pub i18n_string_country_code: i32,
    pub i18n_string_country_subdivision_code: Option<String>,
    pub i18n_string_reference_key: String,
}

impl From<InternationalizationStrings> for InternationalizationStringsToBeEncoded {
    fn from(i18n_string: InternationalizationStrings) -> Self {
        InternationalizationStringsToBeEncoded {
            i18n_string_id: *i18n_string.i18n_string_id.as_bytes(),
            i18n_string_content: i18n_string.i18n_string_content,
            i18n_string_created_at: i18n_string
                .i18n_string_created_at
                .timestamp_millis()
                .to_le_bytes()
                .to_vec(),
            i18n_string_created_by: *i18n_string.i18n_string_created_by.as_bytes(),
            i18n_string_updated_at: i18n_string
                .i18n_string_updated_at
                .timestamp_millis()
                .to_le_bytes()
                .to_vec(),
            i18n_string_updated_by: *i18n_string.i18n_string_updated_by.as_bytes(),
            i18n_string_language_code: i18n_string.i18n_string_language_code,
            i18n_string_country_code: i18n_string.i18n_string_country_code,
            i18n_string_country_subdivision_code: i18n_string.i18n_string_country_subdivision_code,
            i18n_string_reference_key: i18n_string.i18n_string_reference_key,
        }
    }
}

impl InternationalizationStrings {
    pub async fn get_by_id(id: uuid::Uuid, state: &Arc<ServerState>) -> anyhow::Result<Self> {
        let mut conn = state.get_conn().await?;

        let result: InternationalizationStrings = i18n_strings::table
            .filter(i18n_strings::i18n_string_id.eq(id))
            .first::<InternationalizationStrings>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

        drop(conn);

        Ok(result)
    }

    pub async fn get_by_country_and_language_and_subdivision(
        country_code: i32,
        language_code: i32,
        subdivision_code: Option<String>,
        state: &Arc<ServerState>,
    ) -> anyhow::Result<Vec<Self>> {
        let mut conn = state.get_conn().await?;

        let mut query = i18n_strings::table
            .filter(i18n_strings::i18n_string_country_code.eq(country_code))
            .filter(i18n_strings::i18n_string_language_code.eq(language_code))
            .into_boxed();

        if let Some(ref subdivision_code) = subdivision_code {
            query = query
                .filter(i18n_strings::i18n_string_country_subdivision_code.eq(subdivision_code));
        }

        let result: Vec<InternationalizationStrings> = query
            .load::<InternationalizationStrings>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

        drop(conn);

        Ok(result)
    }

    pub async fn get_all(
            mut conn: PooledConnection<'_, AsyncPgConnection>,
        ) -> anyhow::Result<Vec<InternationalizationStrings>> {
            let result: Vec<InternationalizationStrings> = i18n_strings::table
                .load::<InternationalizationStrings>(&mut conn)
                .await
                .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

            drop(conn);

            Ok(result)
        }

    pub async fn get_country_language_bundle(
        country_code: i32,
        language_code: i32,
        state: &Arc<ServerState>,
    ) -> anyhow::Result<Vec<u8>> {
        let country_language_vec: Vec<InternationalizationStrings> =
            Self::get_by_country_and_language_and_subdivision(
                country_code,
                language_code,
                None,
                state,
            )
            .await?;

        let to_be_encoded: Vec<InternationalizationStringsToBeEncoded> = country_language_vec
            .into_iter()
            .map(InternationalizationStringsToBeEncoded::from)
            .collect();

        let encoded = bitcode::encode(&to_be_encoded);

        Ok(encoded)
    }
}
