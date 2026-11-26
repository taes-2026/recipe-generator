//! Stateless GenAI sous-chef proxy: app wiring, HTTP API, domain rules, Gemini.

#![forbid(unsafe_code)]

pub(crate) mod ai;
pub(crate) mod api;
pub(crate) mod app;
pub(crate) mod config;
pub(crate) mod domain;
pub(crate) mod error;

pub use app::{build_router, build_state, AppState};
pub use config::{Config, ConfigError, Mode};
pub use error::ApiError;
