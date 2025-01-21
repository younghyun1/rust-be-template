use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_derive::Serialize;
use std::error::Error;
use std::fmt;
use tracing::Level;

pub type HandlerResult<T> = Result<T, CodeErrorResp>;

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
        log_level: Level::DEBUG,
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

#[derive(Serialize, Debug)]
pub struct CodeErrorResp {
    pub success: bool,
    pub error_code: u16,
    #[serde(serialize_with = "serialize_status_code")]
    pub http_status_code: StatusCode,
    pub message: String,
    pub error_message: String,
    #[serde(skip_serializing)]
    pub log_level: Level,
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

macro_rules! log_with_level {
    ($level:expr, $($arg:tt)*) => {
        match $level {
            Level::ERROR => tracing::error!($($arg)*),
            Level::WARN => tracing::warn!($($arg)*),
            Level::INFO => tracing::info!($($arg)*),
            Level::DEBUG => tracing::debug!($($arg)*),
            Level::TRACE => tracing::trace!($($arg)*),
        }
    };
}

// Ensure CodeErrorResp still implements IntoResponse
impl IntoResponse for CodeErrorResp {
    fn into_response(self) -> axum::response::Response {
        log_with_level!(
            self.log_level,
            "{}: status_code={}, error_code={}, message='{}', error_message='{}'",
            self.log_level.to_string(),
            self.http_status_code,
            self.error_code,
            self.message,
            self.error_message
        );

        let body = serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_string());
        (self.http_status_code, body).into_response()
    }
}
