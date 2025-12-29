use crate::dto::requests::auth::login_request::LoginRequest;
use crate::dto::responses::auth::login_response::LoginResponse;
use crate::dto::responses::photography::get_photograph_response::{
    GetPhotographsResponse, PaginationMeta, PhotographItem,
};
use crate::handlers::auth::login;
use crate::handlers::photography::get_photographs;
use crate::handlers::server::healthcheck;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        healthcheck::healthcheck,
        login::login,
        get_photographs::get_photographs,
    ),
    components(
        schemas(
            LoginRequest,
            LoginResponse,
            GetPhotographsResponse,
            PhotographItem,
            PaginationMeta
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "server", description = "Server status endpoints"),
        (name = "photography", description = "Photography endpoints")
    )
)]
pub struct ApiDoc;
