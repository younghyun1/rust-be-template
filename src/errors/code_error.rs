use axum::Json;
use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;
use serde_derive::Serialize;
use std::error::Error;
use std::fmt::{self, Debug};
use tracing::Level;

pub type HandlerResponse<T> = Result<T, CodeErrorResp>;

#[derive(Copy, Clone, Debug)]
pub struct CodeError {
    pub success: bool,
    pub error_code: u8,
    pub http_status_code: StatusCode,
    pub message: &'static str,
    pub log_level: Level,
}

impl CodeError {
    pub const POOL_ERROR: CodeError = CodeError {
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
    pub const COULD_NOT_HASH_PW: CodeError = CodeError {
        success: false,
        error_code: 4,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Failed to hash the password!",
        log_level: Level::ERROR,
    };
    pub const DB_INSERTION_ERROR: CodeError = CodeError {
        success: false,
        error_code: 5,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Database insertion failed!",
        log_level: Level::ERROR,
    };
    pub const EMAIL_MUST_BE_UNIQUE: CodeError = CodeError {
        success: false,
        error_code: 6,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Email address already exists!",
        log_level: Level::INFO,
    };
    pub const DB_UPDATE_ERROR: CodeError = CodeError {
        success: false,
        error_code: 7,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Database update failed!",
        log_level: Level::ERROR,
    };
    pub const INVALID_EMAIL_VERIFICATION_TOKEN: CodeError = CodeError {
        success: false,
        error_code: 8,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Invalid email verification token!",
        log_level: Level::INFO,
    };
    pub const EMAIL_VERIFICATION_TOKEN_EXPIRED: CodeError = CodeError {
        success: false,
        error_code: 9,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Email verification token has expired!",
        log_level: Level::INFO,
    };
    pub const EMAIL_VERIFICATION_TOKEN_FABRICATED: CodeError = CodeError {
        success: false,
        error_code: 10,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Email verification token was fabricated; created_at was in the future!",
        log_level: Level::ERROR,
    };
    pub const EMAIL_VERIFICATION_TOKEN_ALREADY_USED: CodeError = CodeError {
        success: false,
        error_code: 11,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Email verification token has already been used!",
        log_level: Level::INFO,
    };
    pub const USER_EMAIL_ALREADY_VERIFIED: CodeError = CodeError {
        success: false,
        error_code: 12,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "User email is already verified!",
        log_level: Level::INFO,
    };
    pub const PASSWORD_INVALID: CodeError = CodeError {
        success: false,
        error_code: 13,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Invalid password form! Must contain lower and uppercase characters and digits.",
        log_level: Level::INFO,
    };
    pub const USER_NOT_FOUND: CodeError = CodeError {
        success: false,
        error_code: 14,
        http_status_code: StatusCode::NOT_FOUND,
        message: "User not found!",
        log_level: Level::INFO,
    };
    pub const WRONG_PW: CodeError = CodeError {
        success: false,
        error_code: 15,
        http_status_code: StatusCode::UNAUTHORIZED,
        message: "Incorrect password!",
        log_level: Level::INFO,
    };
    pub const COULD_NOT_VERIFY_PW: CodeError = CodeError {
        success: false,
        error_code: 16,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Wrong password!",
        log_level: Level::INFO,
    };
    pub const SESSION_ID_ALREADY_EXISTS: CodeError = CodeError {
        success: false,
        error_code: 17,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Session ID already exists!",
        log_level: Level::INFO,
    };
    pub const COULD_NOT_REMOVE_OLD_SESSION: CodeError = CodeError {
        success: false,
        error_code: 18,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not remove old session!",
        log_level: Level::ERROR,
    };
    pub const PASSWORD_RESET_TOKEN_FABRICATED: CodeError = CodeError {
        success: false,
        error_code: 19,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Password reset token was fabricated; created_at was in the future!",
        log_level: Level::ERROR,
    };
    pub const PASSWORD_RESET_TOKEN_ALREADY_USED: CodeError = CodeError {
        success: false,
        error_code: 20,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Password reset token has already been used!",
        log_level: Level::INFO,
    };
    pub const PASSWORD_RESET_TOKEN_EXPIRED: CodeError = CodeError {
        success: false,
        error_code: 21,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Password reset token has expired!",
        log_level: Level::INFO,
    };
    pub const UNAUTHORIZED_ACCESS: CodeError = CodeError {
        success: false,
        error_code: 22,
        http_status_code: StatusCode::UNAUTHORIZED,
        message: "Unauthorized access attempt!",
        log_level: Level::WARN,
    };
    pub const POST_TITLE_NOT_UNIQUE: CodeError = CodeError {
        success: false,
        error_code: 23,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Post title must be unique!",
        log_level: Level::INFO,
    };
    pub const POST_NOT_FOUND: CodeError = CodeError {
        success: false,
        error_code: 24,
        http_status_code: StatusCode::NOT_FOUND,
        message: "Post not found or access denied!",
        log_level: Level::INFO,
    };
    pub const INVALID_IP_ADDRESS: CodeError = CodeError {
        success: false,
        error_code: 25,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Invalid IP address provided!",
        log_level: Level::INFO,
    };
    pub const API_KEY_INVALID: CodeError = CodeError {
        success: false,
        error_code: 26,
        http_status_code: StatusCode::UNAUTHORIZED,
        message: "Invalid API key provided!",
        log_level: Level::WARN,
    };
    pub const LANGUAGE_NOT_FOUND: CodeError = CodeError {
        success: false,
        error_code: 27,
        http_status_code: StatusCode::NOT_FOUND,
        message: "Language not found!",
        log_level: Level::INFO,
    };
    pub const COUNTRY_NOT_FOUND: CodeError = CodeError {
        success: false,
        error_code: 28,
        http_status_code: StatusCode::NOT_FOUND,
        message: "Country not found!",
        log_level: Level::INFO,
    };
    pub const UPVOTE_MUST_BE_UNIQUE: CodeError = CodeError {
        success: false,
        error_code: 29,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Upvote must be unique!",
        log_level: Level::INFO,
    };
    pub const DB_DELETION_ERROR: CodeError = CodeError {
        success: false,
        error_code: 30,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Database deletion failed!",
        log_level: Level::ERROR,
    };
    pub const UPVOTE_DOES_NOT_EXIST: CodeError = CodeError {
        success: false,
        error_code: 31,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Upvote does not exist - could not delete.",
        log_level: Level::INFO,
    };
    pub const FILE_UPLOAD_ERROR: CodeError = CodeError {
        success: false,
        error_code: 32,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "File upload failed!",
        log_level: Level::ERROR,
    };
    pub const IMAGE_TOO_LARGE: CodeError = CodeError {
        success: false,
        error_code: 33,
        http_status_code: StatusCode::BAD_REQUEST,
        message: "Image too large! Maximum allowed size is 10MB.",
        log_level: Level::INFO,
    };
    pub const COULD_NOT_PROCESS_IMAGE: CodeError = CodeError {
        success: false,
        error_code: 34,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not process the uploaded image!",
        log_level: Level::ERROR,
    };
    pub const COULD_NOT_CREATE_DIRECTORY: CodeError = CodeError {
        success: false,
        error_code: 35,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not create directory!",
        log_level: Level::ERROR,
    };
    pub const COULD_NOT_WRITE_FILE: CodeError = CodeError {
        success: false,
        error_code: 36,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not write file to disk!",
        log_level: Level::ERROR,
    };
    pub const JOIN_ERROR: CodeError = CodeError {
        success: false,
        error_code: 37,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Failed to perform async task join operation!",
        log_level: Level::ERROR,
    };
    pub const COULD_NOT_GET_I18N_BUNDLE: CodeError = CodeError {
        success: false,
        error_code: 38,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not get i18n bundle!",
        log_level: Level::ERROR,
    };
    pub const COULD_NOT_SYNC_18N_CACHE: CodeError = CodeError {
        success: false,
        error_code: 39,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not sync i18n cache!",
        log_level: Level::ERROR,
    };
    pub const COULD_NOT_RUN_FASTFETCH: CodeError = CodeError {
        success: false,
        error_code: 40,
        http_status_code: StatusCode::INTERNAL_SERVER_ERROR,
        message: "Could not run fastfetch!",
        log_level: Level::ERROR,
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
    pub error_code: u8,
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
