use axum::{body::Body, extract::Request, middleware::Next, response::IntoResponse};

use crate::{
    domain::auth::role::RoleType,
    errors::code_error::{CodeError, HandlerResponse, code_err},
};

#[derive(Clone, Copy, Debug)]
pub enum RoleRequirement {
    AtLeast(RoleType),
}

impl RoleRequirement {
    fn allows(self, role_type: RoleType) -> bool {
        match self {
            RoleRequirement::AtLeast(required_role_type) => role_type.permits(required_role_type),
        }
    }
}

fn require_role(
    request: &Request<Body>,
    role_requirement: RoleRequirement,
) -> HandlerResponse<RoleType> {
    let role_type = match request.extensions().get::<RoleType>().copied() {
        Some(role_type) => role_type,
        None => {
            return Err(code_err(
                CodeError::UNAUTHORIZED_ACCESS,
                "Missing role in request",
            ));
        }
    };

    if role_requirement.allows(role_type) {
        return Ok(role_type);
    }

    Err(code_err(
        CodeError::IS_NOT_SUPERUSER,
        "Insufficient role for this endpoint",
    ))
}

pub async fn require_superuser_middleware(
    request: Request<Body>,
    next: Next,
) -> HandlerResponse<impl IntoResponse> {
    match require_role(&request, RoleRequirement::AtLeast(RoleType::Younghyun)) {
        Ok(_) => Ok(next.run(request).await),
        Err(e) => Err(e),
    }
}
