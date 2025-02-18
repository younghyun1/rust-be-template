use chrono::{DateTime, Utc};
use diesel::{prelude::Insertable, Queryable, QueryableByName};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{email_verification_tokens, password_reset_tokens, users};

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
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'nu> {
    user_name: &'nu str,
    user_email: &'nu str,
    user_password_hash: &'nu str,
}

impl<'nu> NewUser<'nu> {
    pub fn new(user_name: &'nu str, user_email: &'nu str, user_password_hash: &'nu str) -> Self {
        Self {
            user_name,
            user_email,
            user_password_hash,
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
