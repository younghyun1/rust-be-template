use chrono::{DateTime, Utc};
use diesel::{Queryable, QueryableByName, Selectable, prelude::Insertable};
use diesel_async::{
    AsyncConnection, AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection,
    scoped_futures::ScopedFutureExt,
};
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    domain::auth::{role::RoleType, user_roles::UserRole},
    dto::requests::auth::signup_request::SignupRequest,
    errors::code_error::{CodeError, CodeErrorResp, code_err},
    schema::{email_verification_tokens, password_reset_tokens, user_profile_pictures, users},
    util::crypto::hash_pw::hash_pw,
};

// TODO: update with new fields - country, subdivision, etc after filling out the data
#[derive(Serialize, Deserialize, QueryableByName, Queryable, ToSchema)]
#[diesel(table_name = users)]
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

// TODO: update with new fields - country, subdivision, etc after filling out the data
#[derive(Serialize, Deserialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = users)]
pub struct UserInfo {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub user_name: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub user_email: String,
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
// TODO: Separate DTOs and VOs, also separate by response and request
// lmao

impl User {
    pub async fn insert_one(
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
        request: &SignupRequest,
    ) -> Result<Uuid, CodeErrorResp> {
        let new_user = match UserInsertable::from(request).await {
            Ok(new_user) => new_user,
            Err(e) => return Err(code_err(CodeError::COULD_NOT_HASH_PW, e)),
        };

        let inserted_user_id = conn
            .transaction::<Uuid, diesel::result::Error, _>(|conn| {
                async move {
                    let user_id = match diesel::insert_into(users::table)
                        .values(new_user)
                        .returning(users::user_id)
                        .get_result::<Uuid>(conn)
                        .await
                    {
                        Ok(user_id) => user_id,
                        Err(e) => return Err(e),
                    };

                    match UserRole::insert_for_user(conn, user_id, RoleType::User).await {
                        Ok(()) => Ok(user_id),
                        Err(e) => Err(e),
                    }
                }
                .scope_boxed()
            })
            .await;

        match inserted_user_id {
            Ok(user_id) => Ok(user_id),
            Err(e) => match e {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => Err(code_err(CodeError::EMAIL_MUST_BE_UNIQUE, e)),
                _ => Err(code_err(CodeError::DB_INSERTION_ERROR, e)),
            },
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct UserInsertable {
    user_name: String,
    user_email: String,
    user_password_hash: String,
    user_country: i32,
    user_language: i32,
    user_subdivision: Option<i32>,
}

impl UserInsertable {
    async fn from(request: &SignupRequest) -> anyhow::Result<Self> {
        let user_password_hash = match hash_pw(request.user_password.clone()).await {
            Ok(user_password_hash) => user_password_hash,
            Err(e) => return Err(e),
        };

        Ok(Self {
            user_name: request.user_name.clone(),
            user_email: request.user_email.clone(),
            user_password_hash,
            user_country: request.user_country,
            user_language: request.user_language,
            user_subdivision: request.user_subdivision,
        })
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

#[derive(Serialize, Deserialize, QueryableByName, Queryable, ToSchema)]
pub struct UserProfilePicture {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_profile_picture_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub user_profile_picture_created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub user_profile_picture_updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub user_profile_picture_image_type: i32,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub user_profile_picture_is_on_cloud: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub user_profile_picture_link: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = user_profile_pictures)]
pub struct UserProfilePictureInsertable {
    pub user_id: Uuid,
    pub user_profile_picture_image_type: i32,
    pub user_profile_picture_is_on_cloud: bool,
    pub user_profile_picture_link: Option<String>,
}
