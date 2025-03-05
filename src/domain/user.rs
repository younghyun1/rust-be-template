use chrono::{DateTime, Utc};
use diesel::{Queryable, QueryableByName, prelude::Insertable};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::code_error::{CodeError, CodeErrorResp, code_err},
    schema::{email_verification_tokens, password_reset_tokens, users},
};

// TODO: update with new fields - country, subdivision, etc after filling out the data
#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
pub struct User {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub user_name: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub user_email: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub user_password_hash: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub user_created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub user_updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub user_is_email_verified: bool,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub user_country: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub user_language: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    pub user_subdivision: Option<i32>,
}

// TODO: Formalize DTO-Insertion Object relations

impl User {
    pub async fn insert_one<'a, 'conn>(
        conn: &'conn mut PooledConnection<'_, AsyncPgConnection>,
        user_name: &'a str,
        user_email: &'a str,
        user_password_hash: &'a str,
        user_country: i32,
        user_language: i32,
        user_subdivision: Option<i32>,
    ) -> Result<Uuid, CodeErrorResp> {
        let new_user = UserInsertable::new(
            user_name,
            user_email,
            user_password_hash,
            user_country,
            user_language,
            user_subdivision,
        );

        diesel::insert_into(users::table)
            .values(new_user)
            .returning(users::user_id)
            .get_result::<Uuid>(conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => code_err(CodeError::EMAIL_MUST_BE_UNIQUE, e),
                _ => code_err(CodeError::DB_INSERTION_ERROR, e),
            })
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct UserInsertable<'nu> {
    user_name: &'nu str,
    user_email: &'nu str,
    user_password_hash: &'nu str,
    user_country: i32,
    user_language: i32,
    user_subdivision: Option<i32>,
}

impl<'nu> UserInsertable<'nu> {
    pub fn new(
        user_name: &'nu str,
        user_email: &'nu str,
        user_password_hash: &'nu str,
        user_country: i32,
        user_language: i32,
        user_subdivision: Option<i32>,
    ) -> Self {
        Self {
            user_name,
            user_email,
            user_password_hash,
            user_country,
            user_language,
            user_subdivision,
        }
    }
}

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
pub struct EmailVerificationToken {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub email_verification_token_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub email_verification_token: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub email_verification_token_expires_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub email_verification_token_created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub email_verification_token_used_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = email_verification_tokens)]
pub struct NewEmailVerificationToken<'nevt> {
    user_id: &'nevt Uuid,
    email_verification_token: &'nevt Uuid,
    email_verification_token_expires_at: DateTime<Utc>,
    email_verification_token_created_at: DateTime<Utc>,
}

impl<'nevt> NewEmailVerificationToken<'nevt> {
    pub fn new(
        user_id: &'nevt Uuid,
        email_verification_token: &'nevt Uuid,
        email_verification_token_expires_at: DateTime<Utc>,
        email_verification_token_created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            email_verification_token,
            email_verification_token_expires_at,
            email_verification_token_created_at,
        }
    }
}

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
pub struct PasswordResetToken {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub password_reset_token_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub password_reset_token: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub password_reset_token_expires_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub password_reset_token_created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub password_reset_token_used_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = password_reset_tokens)]
pub struct NewPasswordResetToken<'a> {
    user_id: &'a Uuid,
    password_reset_token: &'a Uuid,
    password_reset_token_expires_at: DateTime<Utc>,
    password_reset_token_created_at: DateTime<Utc>,
}

impl<'a> NewPasswordResetToken<'a> {
    pub fn new(
        user_id: &'a Uuid,
        password_reset_token: &'a Uuid,
        password_reset_token_expires_at: DateTime<Utc>,
        password_reset_token_created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            password_reset_token,
            password_reset_token_expires_at,
            password_reset_token_created_at,
        }
    }
}
