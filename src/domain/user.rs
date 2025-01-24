use chrono::{DateTime, Utc};
use diesel::{table, Queryable, QueryableByName};
use serde_derive::{Deserialize, Serialize};

table! {
    users (user_id) {
        user_id -> Uuid,
        user_name -> Varchar,
        user_email -> Varchar,
        user_password_hash -> Varchar,
        user_created_at -> Timestamptz,
        user_updated_at -> Timestamptz,
        user_is_email_verified -> Bool,
    }
}

table! {
    email_verification_tokens (email_verification_token_id) {
        email_verification_token_id -> Uuid,
        user_id -> Uuid,
        email_verification_token -> Uuid,
        email_verification_token_expires_at -> Timestamptz,
        email_verification_token_created_at -> Timestamptz,
    }
}

table! {
    password_reset_tokens (password_reset_token_id) {
        password_reset_token_id -> Uuid,
        user_id -> Uuid,
        password_reset_token -> Uuid,
        password_reset_token_expires_at -> Timestamptz,
        password_reset_token_created_at -> Timestamptz,
    }
}

table! {
    refresh_tokens (refresh_token_id) {
        refresh_token_id -> Uuid,
        user_id -> Uuid,
        refresh_token -> Uuid,
        refresh_token_issued_at -> Timestamptz,
        refresh_token_expires_at -> Timestamptz,
        refresh_token_revoked -> Bool,
    }
}

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
pub struct User {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub user_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub user_email: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub user_password_hash: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub user_created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub user_updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub user_is_email_verified: bool,
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
}

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
pub struct RefreshToken {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub refresh_token_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub refresh_token: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub refresh_token_issued_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub refresh_token_expires_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub refresh_token_revoked: bool,
}
