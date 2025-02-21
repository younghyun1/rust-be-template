// @generated automatically by Diesel CLI.

diesel::table! {
    comments (comment_id) {
        comment_id -> Uuid,
        post_id -> Uuid,
        user_id -> Uuid,
        comment_content -> Text,
        comment_created_at -> Timestamptz,
        comment_updated_at -> Nullable<Timestamptz>,
        parent_comment_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    email_verification_tokens (email_verification_token_id) {
        email_verification_token_id -> Uuid,
        user_id -> Uuid,
        email_verification_token -> Uuid,
        email_verification_token_expires_at -> Timestamptz,
        email_verification_token_created_at -> Timestamptz,
        email_verification_token_used_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    password_reset_tokens (password_reset_token_id) {
        password_reset_token_id -> Uuid,
        user_id -> Uuid,
        password_reset_token -> Uuid,
        password_reset_token_expires_at -> Timestamptz,
        password_reset_token_created_at -> Timestamptz,
        password_reset_token_used_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    permissions (permission_id) {
        permission_id -> Uuid,
        permission_name -> Text,
        permission_description -> Nullable<Text>,
    }
}

diesel::table! {
    posts (post_id) {
        post_id -> Uuid,
        user_id -> Uuid,
        post_title -> Varchar,
        post_slug -> Varchar,
        post_content -> Text,
        post_summary -> Nullable<Text>,
        post_created_at -> Timestamptz,
        post_updated_at -> Timestamptz,
        post_published_at -> Nullable<Timestamptz>,
        post_is_published -> Bool,
    }
}

diesel::table! {
    role_permissions (role_permission_id) {
        role_permission_id -> Uuid,
        role_id -> Uuid,
        permission_id -> Uuid,
    }
}

diesel::table! {
    roles (role_id) {
        role_id -> Uuid,
        role_name -> Text,
        role_description -> Nullable<Text>,
    }
}

diesel::table! {
    user_roles (user_role_id) {
        user_role_id -> Uuid,
        user_id -> Uuid,
        role_id -> Uuid,
    }
}

diesel::table! {
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

diesel::joinable!(comments -> posts (post_id));
diesel::joinable!(comments -> users (user_id));
diesel::joinable!(email_verification_tokens -> users (user_id));
diesel::joinable!(password_reset_tokens -> users (user_id));
diesel::joinable!(posts -> users (user_id));
diesel::joinable!(role_permissions -> permissions (permission_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    comments,
    email_verification_tokens,
    password_reset_tokens,
    permissions,
    posts,
    role_permissions,
    roles,
    user_roles,
    users,
);
