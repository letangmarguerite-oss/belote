//! Erreurs HTTP.
//!
//! Regle : un message precis dans les logs, un message neutre au client. On ne
//! laisse jamais fuir la structure de la base ni l'existence d'un compte.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("ressource introuvable")]
    NotFound,
    #[error("authentification requise")]
    Unauthorized,
    #[error("acces refuse")]
    // Utilise des la Phase 3, quand on refusera l'entree d'une table en cours.
    #[allow(dead_code)]
    Forbidden,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = match &self {
            // Le detail interne reste dans les logs.
            ApiError::Internal(err) => {
                tracing::error!(error = ?err, "erreur interne");
                "erreur interne".to_string()
            }
            other => other.to_string(),
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound,
            other => ApiError::Internal(other.into()),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
