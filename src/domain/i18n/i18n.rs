use std::sync::Arc;

use diesel::{
    ExpressionMethods, QueryDsl,
    prelude::{Insertable, Queryable, QueryableByName},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use serde_derive::{Deserialize, Serialize};

use crate::{
    errors::code_error::{CodeError, code_err},
    init::state::ServerState,
    schema::i18n_strings,
};

#[derive(Clone, Serialize, Deserialize, QueryableByName, Queryable, Insertable)]
#[diesel(table_name = i18n_strings)]
pub struct InternationalizationString {
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

impl InternationalizationString {
    pub async fn get_by_id(id: uuid::Uuid, state: &Arc<ServerState>) -> anyhow::Result<Self> {
        let mut conn = state.get_conn().await?;

        let result: InternationalizationString = i18n_strings::table
            .filter(i18n_strings::i18n_string_id.eq(id))
            .first::<InternationalizationString>(&mut conn)
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

        let result: Vec<InternationalizationString> = query
            .load::<InternationalizationString>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

        drop(conn);

        Ok(result)
    }

    pub async fn get_all(
        mut conn: PooledConnection<'_, AsyncPgConnection>,
    ) -> anyhow::Result<Vec<InternationalizationString>> {
        let result: Vec<InternationalizationString> = i18n_strings::table
            .load::<InternationalizationString>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

        drop(conn);

        Ok(result)
    }
}
