// use std::{net::SocketAddr, sync::Arc};

// use axum::{
//     body::Body,
//     extract::{ConnectInfo, Request, State},
//     http::Response,
//     middleware::Next,
// };

// use crate::init::state::ServerState;

// pub async fn auth_middleware(
//     State(state): State<Arc<ServerState>>,
//     ConnectInfo(info): ConnectInfo<SocketAddr>,
//     mut request: Request<Body>,
//     next: Next,
// ) -> Response<Body> {
//     // Extract the "Cookie" header
//     let cookie_header = if let Some(c) = request.headers().get("Cookie") {
//         c.to_str().unwrap_or("")
//     } else {
//         return Response::builder()
//             .status(401)
//             .body(Body::from("Unauthorized: no cookie header"))
//             .unwrap();
//     };

//     // Look for the session_id in the cookie header
//     let session_id = cookie_header
//         .split(';')
//         .map(|s| s.trim())
//         .find_map(|cookie| {
//             if cookie.starts_with("session_id=") {
//                 Some(cookie.trim_start_matches("session_id="))
//             } else {
//                 None
//             }
//         });

//     let session_id = if let Some(sid) = session_id {
//         sid
//     } else {
//         return Response::builder()
//             .status(401)
//             .body(Body::from("Unauthorized: session_id cookie missing"))
//             .unwrap();
//     };

//     // Retrieve the session from state.
//     // Assuming state.sessions is a map-like structure that holds session data
//     let session = if let Some(s) = state.sessions.get(session_id) {
//         s
//     } else {
//         return Response::builder()
//             .status(401)
//             .body(Body::from("Unauthorized: invalid session"))
//             .unwrap();
//     };

//     // Extract the user email from the request headers for additional verification.
//     // Assuming the email is sent in a header called "X-User-Email"
//     let header_email = if let Some(email_val) = request.headers().get("X-User-Email") {
//         email_val.to_str().unwrap_or("")
//     } else {
//         return Response::builder()
//             .status(401)
//             .body(Body::from("Unauthorized: missing user email header"))
//             .unwrap();
//     };

//     // Check if the email from the header matches the one stored in session.
//     if header_email != session.user_email {
//         return Response::builder()
//             .status(401)
//             .body(Body::from("Unauthorized: user email mismatch"))
//             .unwrap();
//     }

//     // Proceed to the next middleware/controller if everything checks out.
//     next.run(request).await
// }
