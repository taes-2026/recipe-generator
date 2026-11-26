use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Variants map 1:1 to the stable code strings the frontend switches on.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid request")]
    InvalidInput(Vec<String>),
    #[error("sous-chef upstream failed: {0}")]
    Upstream(String),
    #[error("sous-chef upstream is rate limited")]
    UpstreamRateLimited,
    #[error("sous-chef upstream timed out")]
    UpstreamTimeout,
    #[error("the model refused to answer")]
    UpstreamRefused(String),
    #[error("model output rejected")]
    InvalidModelOutput(Vec<String>),
    #[error("sous-chef is misconfigured")]
    Misconfigured(String),
    #[error("too many requests")]
    RateLimited,
    #[error("internal error")]
    Internal(String),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::Upstream(_) | Self::InvalidModelOutput(_) => StatusCode::BAD_GATEWAY,
            Self::UpstreamRateLimited | Self::UpstreamRefused(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::UpstreamTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Misconfigured(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Upstream(_) => "upstream_error",
            Self::UpstreamRateLimited => "upstream_rate_limited",
            Self::UpstreamTimeout => "upstream_timeout",
            Self::UpstreamRefused(_) => "upstream_refused",
            Self::InvalidModelOutput(_) => "invalid_model_output",
            Self::Misconfigured(_) => "misconfigured",
            Self::RateLimited => "rate_limited",
            Self::Internal(_) => "internal",
        }
    }

    /// Only our own messages: never the API key or raw model output.
    pub fn details(&self) -> Option<Vec<String>> {
        match self {
            Self::InvalidInput(details) | Self::InvalidModelOutput(details) => {
                Some(details.clone())
            }
            _ => None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if matches!(self, Self::Internal(_) | Self::Misconfigured(_)) {
            tracing::error!(error = %self, "sous-chef request failed");
        }

        let status = self.status();
        let body = json!({
            "error": {
                "code": self.code(),
                "message": self.to_string(),
                "details": self.details(),
            }
        });

        (status, Json(body)).into_response()
    }
}
