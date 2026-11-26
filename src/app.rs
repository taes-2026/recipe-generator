use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method},
    routing::{get, post},
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::{
    ai::{Provider, SousChefService},
    api::{routes, RateLimiter},
    config::{Config, ConfigError},
};

const MAX_BODY_BYTES: usize = 32 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub(crate) config: Arc<Config>,
    pub(crate) service: Arc<SousChefService>,
    pub(crate) limiter: RateLimiter,
}

pub fn build_state(config: Config) -> Result<AppState, ConfigError> {
    let provider = Provider::from_config(&config)?;
    let service = SousChefService::new(provider, config.max_attempts);

    Ok(AppState {
        config: Arc::new(config),
        service: Arc::new(service),
        limiter: RateLimiter::default(),
    })
}

#[must_use = "the router is what the server serves"]
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(state.config.allowed_origins.clone()))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/healthz", get(routes::healthz))
        .route("/v1/sous-chef", post(routes::sous_chef))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
