// use std::sync::Arc;

// use axum::{extract::State, response::IntoResponse};
// use axum_extra::extract::cookie::{Cookie, CookieJar};

// use uuid::Uuid;

// use crate::{
//     dto::responses::response_data::http_resp_with_cookies, errors::code_error::HandlerResponse,
//     init::state::ServerState, util::time::now::tokio_now,
// };

// pub async fn logout(
//     State(state): State<Arc<ServerState>>,
//     jar: CookieJar,
// ) -> HandlerResponse<impl IntoResponse> {
//     let start = tokio_now();

//     // Try to obtain the session id from the cookie jar.
//     if let Some(cookie) = jar.get("session_id") {
//         if let Ok(session_uuid) = Uuid::parse_str(cookie.value()) {
//             // Remove the session from the session_map (ignore error if not found).
//             let _ = state.session_map.remove_async(session_uuid).await;
//         }
//     }

//     // Build a cookie that instructs the browser to clear the "session_id" cookie.
//     // (The response responder will call .make_removal() on cookies in cookies_to_unset.)
//     let clear_cookie = Cookie::build(("session_id", ""))
//         .path("/")
//         .http_only(true)
//         .secure(true)
//         .same_site(axum_extra::extract::cookie::SameSite::Strict)
//         .build();

//     Ok(http_resp_with_cookies(
//         serde_json::json!({ "message": "Logout successful" }),
//         (),
//         start,
//         None,
//         Some(vec![clear_cookie]),
//     ))
// }
