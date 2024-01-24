use axum::{
    http::{self, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug)]
pub struct AppError {
    message: String,
    code: http::StatusCode,
}

impl AppError {
    pub fn new(code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<serde_json::error::Error> for AppError {
    fn from(error: serde_json::error::Error) -> Self {
        AppError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Serde JSON Error:\n{}", error),
        }
    }
}

// TODO: Pattern match for all error types with their respective status codes
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        let code: StatusCode;

        match error {
            sqlx::Error::RowNotFound | sqlx::Error::ColumnNotFound(_) => {
                code = StatusCode::NOT_FOUND
            }
            sqlx::Error::TypeNotFound { type_name: _ } => code = StatusCode::NOT_FOUND,
            sqlx::Error::Database(ref db_err) => match db_err.kind() {
                sqlx::error::ErrorKind::UniqueViolation => code = StatusCode::CONFLICT,
                sqlx::error::ErrorKind::NotNullViolation
                | sqlx::error::ErrorKind::ForeignKeyViolation => code = StatusCode::BAD_REQUEST,
                sqlx::error::ErrorKind::CheckViolation => code = StatusCode::UNPROCESSABLE_ENTITY,
                _ => code = StatusCode::INTERNAL_SERVER_ERROR,
            },
            sqlx::Error::ColumnDecode {
                index: _,
                source: _,
            } => code = StatusCode::UNPROCESSABLE_ENTITY,
            _ => code = StatusCode::INTERNAL_SERVER_ERROR,
        }

        AppError {
            code,
            message: format!("SQLx Error:\n{}", error),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Anyhow Error:\n{}", error),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("{} {:<12} - {}", "ERROR", self.code, self.message);

        (self.code, self.message).into_response()
    }
}
