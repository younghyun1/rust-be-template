//! OpenAPI documentation registration for Swagger UI.
//!
//! Important: Utoipa only exposes operations you list in `#[openapi(paths(...))]`.
//! Handler functions still need their own `#[utoipa::path(...)]` attributes.

use utoipa::OpenApi;

// ---- handlers (for `paths(...)`) ----
use crate::handlers::{
    admin::sync_i18n_cache,
    auth::{
        check_if_user_exists, is_superuser, login, logout, me, reset_password,
        reset_password_request, signup, verify_user_email,
    },
    blog::{
        delete_comment, delete_post, get_posts, read_post, rescind_comment_vote, rescind_post_vote,
        submit_comment, submit_post, update_comment, update_post, vote_comment, vote_post,
    },
    countries::{
        get_countries, get_country, get_language, get_languages, get_subdivisions_for_country,
    },
    geo_ip::lookup_ip,
    i18n::get_country_language_bundle,
    photography::{delete_photographs, get_photographs, upload_photograph},
    server::{get_host_fastfetch, healthcheck, lookup_ip_loc, root, visitor_board},
    user::upload_profile_picture,
};

// ---- schemas (for `components(schemas(...))`) ----
use crate::domain::{
    auth::user::{User, UserInfo, UserProfilePicture},
    blog::blog::{
        Comment, CommentResponse, Post, PostInfo, PostInfoWithVote, Tag, UserBadgeInfo, VoteState,
    },
    country::{
        CountryAndSubdivisions, IsoCountry, IsoCountrySubdivision, IsoCurrency, IsoLanguage,
    },
    photography::photographs::Photograph,
};
use crate::dto::{
    requests::{
        auth::{
            check_if_user_exists_request::CheckIfUserExistsRequest, login_request::LoginRequest,
            reset_password::ResetPasswordProcessRequest,
            reset_password_request::ResetPasswordRequest, signup_request::SignupRequest,
            verify_user_email_request::EmailValidationToken,
        },
        blog::{
            get_posts_request::GetPostsRequest, submit_comment::SubmitCommentRequest,
            submit_post_request::SubmitPostRequest, update_comment_request::UpdateCommentRequest,
            update_post_request::UpdatePostRequest, upvote_comment_request::UpvoteCommentRequest,
            upvote_post_request::UpvotePostRequest,
        },
        i18n::get_country_language_bundle_request::GetCountryLanguageBundleRequest,
        photography::delete_photographs_request::DeletePhotographsRequest,
    },
    responses::{
        admin::sync_i18n_cache_response::SyncI18nCacheResponse,
        auth::{
            is_superuser_response::IsSuperuserResponse, login_response::LoginResponse,
            logout_response::LogoutResponse, me_response::MeResponse,
            reset_password_request_response::ResetPasswordRequestResponse,
            reset_password_response::ResetPasswordResponse, signup_response::SignupResponse,
        },
        blog::{
            delete_comment_response::DeleteCommentResponse,
            delete_post_response::DeletePostResponse, get_posts::GetPostsResponse,
            read_post_response::ReadPostResponse, submit_post_response::SubmitPostResponse,
            vote_comment_response::VoteCommentResponse, vote_post_response::VotePostResponse,
        },
        photography::get_photograph_response::{
            GetPhotographsResponse, PaginationMeta, PhotographItem,
        },
    },
};
use crate::errors::code_error::CodeErrorResp;
use crate::util::geographic::ip_info_lookup::IpInfo;

/// Central OpenAPI document for Swagger UI.
#[derive(OpenApi)]
#[openapi(
    // All public + protected API routes from `main_router.rs`.
    paths(
        // --- server ---
        healthcheck::healthcheck,
        root::root_handler,
        get_host_fastfetch::get_host_fastfetch,
        visitor_board::get_visitor_board_entries,
        lookup_ip_loc::lookup_ip_location,

        // --- geo_ip ---
        lookup_ip::lookup_ip_info,

        // --- dropdowns / countries ---
        get_languages::get_languages,
        get_language::get_language,
        get_countries::get_countries,
        get_country::get_country,
        get_subdivisions_for_country::get_subdivisions_for_country,

        // --- auth ---
        signup::signup_handler,
        is_superuser::is_superuser_handler,
        me::me_handler,
        check_if_user_exists::check_if_user_exists_handler,
        login::login,
        reset_password_request::reset_password_request_process,
        reset_password::reset_password,
        verify_user_email::verify_user_email,
        logout::logout,

        // --- blog ---
        get_posts::get_posts,
        read_post::read_post,
        submit_post::submit_post,
        vote_post::vote_post,
        vote_comment::vote_comment,
        rescind_post_vote::rescind_post_vote,
        delete_comment::delete_comment,
        update_comment::update_comment,
        delete_post::delete_post,
        update_post::update_post,
        submit_comment::submit_comment,
        rescind_comment_vote::rescind_comment_vote,

        // --- i18n ---
        get_country_language_bundle::get_country_language_bundle,

        // --- admin ---
        sync_i18n_cache::sync_i18n_cache,

        // --- photography ---
        get_photographs::get_photographs,
        upload_photograph::upload_photograph,
        delete_photographs::delete_photographs,

        // --- user ---
        upload_profile_picture::upload_profile_picture,
    ),
    components(
        schemas(
            // shared error response
            CodeErrorResp,

            // --- auth DTOs ---
            SignupRequest,
            SignupResponse,
            CheckIfUserExistsRequest,
            LoginRequest,
            LoginResponse,
            LogoutResponse,
            MeResponse,
            IsSuperuserResponse,
            ResetPasswordRequest,
            ResetPasswordRequestResponse,
            ResetPasswordProcessRequest,
            ResetPasswordResponse,
            EmailValidationToken,

            // --- blog DTOs ---
            GetPostsRequest,
            GetPostsResponse,
            ReadPostResponse,
            SubmitPostRequest,
            SubmitPostResponse,
            UpvotePostRequest,
            VotePostResponse,
            UpvoteCommentRequest,
            VoteCommentResponse,
            SubmitCommentRequest,
            UpdateCommentRequest,
            UpdatePostRequest,
            DeleteCommentResponse,
            DeletePostResponse,

            // --- i18n DTOs ---
            GetCountryLanguageBundleRequest,

            // --- admin DTOs ---
            SyncI18nCacheResponse,

            // --- photography DTOs ---
            GetPhotographsResponse,
            PhotographItem,
            PaginationMeta,
            DeletePhotographsRequest,

            // --- domain models used in responses ---
            IpInfo,

            IsoCountry,
            IsoCountrySubdivision,
            IsoCurrency,
            IsoLanguage,
            CountryAndSubdivisions,

            User,
            UserInfo,
            UserProfilePicture,

            Post,
            PostInfo,
            PostInfoWithVote,
            Comment,
            CommentResponse,
            Tag,
            UserBadgeInfo,
            VoteState,

            Photograph,
        )
    ),
    tags(
        (name = "server", description = "Server status endpoints"),
        (name = "geo", description = "Geo / IP lookup endpoints"),
        (name = "countries", description = "Dropdown / country-language endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "blog", description = "Blog endpoints"),
        (name = "i18n", description = "Internationalization endpoints"),
        (name = "admin", description = "Admin endpoints"),
        (name = "photography", description = "Photography endpoints"),
        (name = "user", description = "User endpoints")
    )
)]
pub struct ApiDoc;
