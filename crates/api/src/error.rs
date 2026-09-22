use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<codeatlas_indexer::IndexError> for ApiError {
    fn from(error: codeatlas_indexer::IndexError) -> Self {
        Self::bad_request(error.to_string())
    }
}

impl From<codeatlas_storage::StorageError> for ApiError {
    fn from(error: codeatlas_storage::StorageError) -> Self {
        Self::internal(error.to_string())
    }
}

impl From<codeatlas_search::SearchError> for ApiError {
    fn from(error: codeatlas_search::SearchError) -> Self {
        Self::internal(error.to_string())
    }
}

impl From<codeatlas_git::GitError> for ApiError {
    fn from(error: codeatlas_git::GitError) -> Self {
        Self::internal(error.to_string())
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<codeatlas_ai::AiError> for ApiError {
    fn from(error: codeatlas_ai::AiError) -> Self {
        match error {
            codeatlas_ai::AiError::EmptyContext => Self::bad_request(error.to_string()),
            _ => Self::internal(error.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: String,
        }

        (self.status, Json(Body { error: self.message })).into_response()
    }
}
