use std::net::SocketAddr;

use recipe_generator::{build_router, build_state, Config};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("recipe_generator=info,tower_http=info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("configuration error: {error}");
            std::process::exit(2);
        }
    };

    let addr = config.bind_addr;
    let mode = config.mode;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        ?mode,
        model = %config.gemini_model,
        addr = %addr,
        timeout_ms = config.upstream_timeout.as_millis() as u64,
        max_attempts = config.max_attempts,
        rate_limit_per_minute = config.rate_limit_per_minute,
        allowed_origins = config.allowed_origins.len(),
        api_key_configured = config.gemini_api_key.is_some(),
        "sous-chef starting"
    );

    let state = match build_state(config) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("configuration error: {error}");
            std::process::exit(2);
        }
    };

    let app = build_router(state);
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("could not bind {addr}: {error}");
            std::process::exit(1);
        }
    };

    tracing::info!(%addr, ?mode, "sous-chef listening");

    if let Err(error) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        eprintln!("server error: {error}");
        std::process::exit(1);
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
