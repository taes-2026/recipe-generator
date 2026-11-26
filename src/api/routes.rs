use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{rejection::JsonRejection, ConnectInfo, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

use crate::{
    domain::{GenerateRequest, GenerateResponse},
    ApiError, AppState, Mode,
};

pub async fn healthz(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "mode": match state.config.mode {
            Mode::Gemini => "gemini",
            Mode::Testing => "testing",
        },
    }))
}

pub async fn sous_chef(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<GenerateRequest>, JsonRejection>,
) -> Result<Json<GenerateResponse>, ApiError> {
    let key = client_key(&headers, addr);

    if !state
        .limiter
        .check(&key, state.config.rate_limit_per_minute, now_secs())
    {
        tracing::warn!(
            client = %key,
            limit = state.config.rate_limit_per_minute,
            "rate limit exceeded"
        );
        return Err(ApiError::RateLimited);
    }

    let Json(request) = body.map_err(|rejection| {
        tracing::warn!(body = %rejection.body_text(), "malformed request body");
        ApiError::InvalidInput(vec![format!(
            "malformed JSON body: {}",
            rejection.body_text()
        )])
    })?;

    tracing::debug!(
        servings = request.profile.servings,
        leftovers = request.ingredients.len(),
        pantry = request.pantry.len(),
        diet = %request.profile.diet,
        max_minutes = request.max_minutes,
        "request received"
    );

    let response = state.service.generate(request).await?;

    Ok(Json(response))
}

fn client_key(headers: &HeaderMap, addr: SocketAddr) -> String {
    for header in ["x-forwarded-for", "x-real-ip"] {
        if let Some(value) = headers.get(header).and_then(|value| value.to_str().ok()) {
            if let Some(first) = value
                .split(',')
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return first.to_owned();
            }
        }
    }

    addr.ip().to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
