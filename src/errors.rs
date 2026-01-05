use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error; // Recommended for clean error definitions

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Wrong credentials")]
    WrongCredentials,

    #[error("Missing credentials")]
    MissingCredentials,

    #[error("Token creation error")]
    TokenCreation,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::UserAlreadyExists => (
                StatusCode::CONFLICT,
                "A user with this email already exists",
            ),
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Invalid email or password"),
            AuthError::MissingCredentials => {
                (StatusCode::BAD_REQUEST, "Email and password are required")
            }
            AuthError::TokenCreation => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate session",
            ),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired session"),
            AuthError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred",
            ),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
