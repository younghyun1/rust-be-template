// @generated automatically by Diesel CLI.

diesel::table! {
    comment_votes (vote_id) {
        vote_id -> Uuid,
        comment_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
        is_upvote -> Bool,
    }
}

diesel::table! {
    comments (comment_id) {
        comment_id -> Uuid,
        post_id -> Uuid,
        user_id -> Uuid,
        comment_content -> Text,
        comment_created_at -> Timestamptz,
        comment_updated_at -> Nullable<Timestamptz>,
        parent_comment_id -> Nullable<Uuid>,
        total_upvotes -> Int8,
        total_downvotes -> Int8,
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
    i18n_strings (i18n_string_id) {
        i18n_string_id -> Uuid,
        i18n_string_content -> Varchar,
        i18n_string_created_at -> Timestamptz,
        i18n_string_created_by -> Uuid,
        i18n_string_updated_at -> Timestamptz,
        i18n_string_updated_by -> Uuid,
        i18n_string_language_code -> Int4,
        i18n_string_country_code -> Int4,
        i18n_string_country_subdivision_code -> Nullable<Varchar>,
        i18n_string_reference_key -> Varchar,
    }
}

diesel::table! {
    iso_country (country_code) {
        country_code -> Int4,
        #[max_length = 2]
        country_alpha2 -> Bpchar,
        #[max_length = 3]
        country_alpha3 -> Bpchar,
        #[max_length = 255]
        country_eng_name -> Varchar,
        country_currency -> Int4,
        #[max_length = 10]
        phone_prefix -> Varchar,
        #[max_length = 2]
        country_flag -> Bpchar,
        is_country -> Bool,
        country_primary_language -> Int4,
    }
}

diesel::table! {
    iso_country_subdivision (subdivision_id) {
        subdivision_id -> Int4,
        country_code -> Int4,
        #[max_length = 10]
        subdivision_code -> Varchar,
        #[max_length = 255]
        subdivision_name -> Varchar,
        #[max_length = 50]
        subdivision_type -> Nullable<Varchar>,
    }
}

diesel::table! {
    iso_currency (currency_code) {
        currency_code -> Int4,
        #[max_length = 3]
        currency_alpha3 -> Bpchar,
        #[max_length = 255]
        currency_name -> Varchar,
    }
}

diesel::table! {
    iso_language (language_code) {
        language_code -> Int4,
        #[max_length = 2]
        language_alpha2 -> Bpchar,
        #[max_length = 3]
        language_alpha3 -> Bpchar,
        #[max_length = 255]
        language_eng_name -> Varchar,
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
    post_tags (post_id, tag_id) {
        post_id -> Uuid,
        tag_id -> Int2,
    }
}

diesel::table! {
    post_votes (vote_id) {
        vote_id -> Uuid,
        post_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
        is_upvote -> Bool,
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
        post_view_count -> Int8,
        post_share_count -> Int8,
        post_metadata -> Jsonb,
        total_upvotes -> Int8,
        total_downvotes -> Int8,
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
    tags (tag_id) {
        tag_id -> Int2,
        tag_name -> Varchar,
    }
}

diesel::table! {
    user_profile_picture_image_types (image_type_id) {
        image_type_id -> Int4,
        image_type_name -> Varchar,
    }
}

diesel::table! {
    user_profile_pictures (user_profile_picture_id) {
        user_profile_picture_id -> Uuid,
        user_id -> Uuid,
        user_profile_picture_created_at -> Timestamptz,
        user_profile_picture_updated_at -> Timestamptz,
        user_profile_picture_image_type -> Int4,
        user_profile_picture_is_on_cloud -> Bool,
        user_profile_picture_link -> Nullable<Varchar>,
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
        user_country -> Int4,
        user_language -> Int4,
        user_subdivision -> Nullable<Int4>,
    }
}

diesel::joinable!(comment_votes -> comments (comment_id));
diesel::joinable!(comment_votes -> users (user_id));
diesel::joinable!(comments -> posts (post_id));
diesel::joinable!(comments -> users (user_id));
diesel::joinable!(email_verification_tokens -> users (user_id));
diesel::joinable!(i18n_strings -> iso_country (i18n_string_country_code));
diesel::joinable!(i18n_strings -> iso_language (i18n_string_language_code));
diesel::joinable!(iso_country -> iso_currency (country_currency));
diesel::joinable!(iso_country -> iso_language (country_primary_language));
diesel::joinable!(iso_country_subdivision -> iso_country (country_code));
diesel::joinable!(password_reset_tokens -> users (user_id));
diesel::joinable!(post_tags -> posts (post_id));
diesel::joinable!(post_tags -> tags (tag_id));
diesel::joinable!(post_votes -> posts (post_id));
diesel::joinable!(post_votes -> users (user_id));
diesel::joinable!(posts -> users (user_id));
diesel::joinable!(role_permissions -> permissions (permission_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(user_profile_pictures -> user_profile_picture_image_types (user_profile_picture_image_type));
diesel::joinable!(user_profile_pictures -> users (user_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));
diesel::joinable!(users -> iso_country (user_country));
diesel::joinable!(users -> iso_country_subdivision (user_subdivision));
diesel::joinable!(users -> iso_language (user_language));

diesel::allow_tables_to_appear_in_same_query!(
    comment_votes,
    comments,
    email_verification_tokens,
    i18n_strings,
    iso_country,
    iso_country_subdivision,
    iso_currency,
    iso_language,
    password_reset_tokens,
    permissions,
    post_tags,
    post_votes,
    posts,
    role_permissions,
    roles,
    tags,
    user_profile_picture_image_types,
    user_profile_pictures,
    user_roles,
    users,
);
