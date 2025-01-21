use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde_derive::Serialize;
use std::error::Error;
use std::fmt::{self, Debug};
use tracing::Level;

pub type HandlerResult<T> = Result<T, CodeErrorResp>;

#[derive(Copy, Clone, Debug)]
pub struct CodeError {
    pub success: bool,
    pub error_code: u16,
    pub http_status_code: StatusCode,
    pub message: &'static str,
    pub log_level: Level,
}

impl CodeError {
    pub const DB_CONNECTION_ERROR: CodeError = CodeError {
        success: false,
        error_code: 0,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not get conn out of pool!",
        log_level: Level::ERROR,
    };
    pub const DB_QUERY_ERROR: CodeError = CodeError {
        success: false,
        error_code: 1,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Database query failed!",
        log_level: Level::ERROR,
    };
    pub const EMAIL_INVALID: CodeError = CodeError {
        success: false,
        error_code: 2,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Invalid email address!",
        log_level: Level::INFO, // info, debug, trace all info'd
    };
    pub const USER_NAME_INVALID: CodeError = CodeError {
        success: false,
        error_code: 3,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Invalid username!",
        log_level: Level::INFO,
    };
}

pub fn code_err(cerr: CodeError, e: impl ToString) -> CodeErrorResp {
    CodeErrorResp {
        success: cerr.success,
        error_code: cerr.error_code,
        http_status_code: cerr.http_status_code,
        message: cerr.message.to_string(),
        error_message: e.to_string(),
        log_level: cerr.log_level,
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct CodeErrorResp {
    pub success: bool,
    pub error_code: u16,
    #[serde(skip_serializing)]
    pub http_status_code: StatusCode,
    pub message: String,
    #[serde(skip_serializing)]
    pub error_message: String,
    #[serde(skip_serializing)]
    pub log_level: Level,
}

// Implement std::fmt::Display for CodeErrorResp
impl fmt::Display for CodeErrorResp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.error_message)
    }
}

// Implement std::error::Error for CodeErrorResp
impl Error for CodeErrorResp {}

// Implement IntoResponse for CodeErrorResp
impl IntoResponse for CodeErrorResp {
    fn into_response(self) -> axum::response::Response {
        let body = Json(&self);
        let mut response = (self.http_status_code, body).into_response();

        response.headers_mut().insert(
            "X-Error-Log-Level",
            HeaderValue::from_str(&self.log_level.to_string()).unwrap(),
        );
        response.headers_mut().insert(
            "X-Error-Status-Code",
            HeaderValue::from_str(&self.http_status_code.as_u16().to_string()).unwrap(),
        );
        response.headers_mut().insert(
            "X-Error-Code",
            HeaderValue::from_str(&self.error_code.to_string()).unwrap(),
        );
        response.headers_mut().insert(
            "X-Error-Message",
            HeaderValue::from_str(&self.message).unwrap(),
        );
        response.headers_mut().insert(
            "X-Error-Detail",
            HeaderValue::from_str(&self.error_message).unwrap(),
        );

        response
    }
}

// Implement From<CodeError> for CodeErrorResp
impl From<CodeError> for CodeErrorResp {
    fn from(cerr: CodeError) -> Self {
        CodeErrorResp {
            success: cerr.success,
            error_code: cerr.error_code,
            http_status_code: cerr.http_status_code,
            message: cerr.message.to_string(),
            error_message: "".to_string(),
            log_level: cerr.log_level,
        }
    }
}

// Implement IntoResponse for CodeError
impl IntoResponse for CodeError {
    fn into_response(self) -> axum::response::Response {
        let resp: CodeErrorResp = self.into();
        resp.into_response()
    }
}
