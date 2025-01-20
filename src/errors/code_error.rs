use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_derive::Serialize;
use std::error::Error;
use std::fmt;

pub type HandlerResult<T> = Result<T, CodeErrorResp>;

pub struct CodeError {
    pub success: bool,
    pub error_code: u16,
    pub http_status_code: StatusCode,
    pub message: &'static str,
}

impl CodeError {
    pub const DB_CONNECTION_ERROR: CodeError = CodeError {
        success: false,
        error_code: 0,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not get conn out of pool!",
    };
    pub const DB_QUERY_ERROR: CodeError = CodeError {
        success: false,
        error_code: 1,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Database query failed!",
    };
}

// Update the trait bound to std::fmt::Display
pub fn code_err(cerr: CodeError, e: anyhow::Error) -> CodeErrorResp {
    CodeErrorResp {
        success: cerr.success,
        error_code: cerr.error_code,
        http_status_code: cerr.http_status_code,
        message: cerr.message.to_string(),
        error_message: e.to_string(),
    }
}

#[derive(Serialize, Debug)]
pub struct CodeErrorResp {
    pub success: bool,
    pub error_code: u16,
    #[serde(serialize_with = "serialize_status_code")]
    pub http_status_code: StatusCode,
    pub message: String,
    pub error_message: String,
}

fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u16(status.as_u16())
}

// Implement std::fmt::Display for CodeErrorResp
impl fmt::Display for CodeErrorResp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.error_message)
    }
}

// Implement std::error::Error for CodeErrorResp
impl Error for CodeErrorResp {}

// Ensure CodeErrorResp still implements IntoResponse
impl IntoResponse for CodeErrorResp {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(
            "Error occurred: status_code={}, error_code={}, message='{}', error_message='{}'",
            self.http_status_code,
            self.error_code,
            self.message,
            self.error_message
        );
        let body = serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_string());
        (self.http_status_code, body).into_response()
    }
}
