use axum::extract::FromRequestParts;
use axum::http::{StatusCode, header::HOST, request::Parts};

#[derive(Clone, Debug)]
pub struct Host(pub String);

impl<S> FromRequestParts<S> for Host
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let header_host = parts
            .headers
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        let authority_host = parts
            .uri
            .authority()
            .map(|value| value.as_str().to_string());

        let result = if let Some(host) = header_host {
            Ok(Host(host))
        } else if let Some(host) = authority_host {
            Ok(Host(host))
        } else {
            Err((StatusCode::BAD_REQUEST, "Missing Host header"))
        };

        std::future::ready(result)
    }
}
