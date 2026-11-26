use std::{
    env,
    fmt::Display,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use axum::http::HeaderValue;
use thiserror::Error;

const DEFAULT_PORT: u16 = 8787;
const MIN_TIMEOUT_MS: u64 = 1_000;
const MAX_TIMEOUT_MS: u64 = 120_000;
const MIN_ATTEMPTS: u8 = 1;
const MAX_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Gemini,
    Testing,
}

impl Mode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "gemini" | "real" | "live" => Some(Self::Gemini),
            "testing" | "mock" | "test" => Some(Self::Testing),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid value for {var}: {message}")]
    Invalid { var: &'static str, message: String },
    #[error("GEMINI_API_KEY is required when SOUS_CHEF_MODE=gemini")]
    MissingApiKey,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub allowed_origins: Vec<HeaderValue>,
    pub mode: Mode,
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub gemini_base_url: String,
    /// Omitted by default: Gemini 3.x flash-lite rejects thinkingBudget with a 400.
    pub gemini_thinking_budget: Option<u32>,
    pub upstream_timeout: Duration,
    pub max_attempts: u8,
    pub rate_limit_per_minute: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_PORT),
            allowed_origins: vec![
                HeaderValue::from_static("http://localhost:5173"),
                HeaderValue::from_static("http://127.0.0.1:5173"),
            ],
            mode: Mode::Testing,
            gemini_api_key: None,
            gemini_model: "gemini-3.5-flash-lite".to_owned(),
            gemini_base_url: "https://generativelanguage.googleapis.com".to_owned(),
            gemini_thinking_budget: None,
            upstream_timeout: Duration::from_millis(25_000),
            max_attempts: 3,
            rate_limit_per_minute: 10,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let api_key = env_value("GEMINI_API_KEY");

        config.mode = match env_value("SOUS_CHEF_MODE") {
            Some(raw) => Mode::parse(&raw).ok_or_else(|| ConfigError::Invalid {
                var: "SOUS_CHEF_MODE",
                message: format!("expected 'gemini' or 'testing', got '{raw}'"),
            })?,
            None if api_key.is_some() => Mode::Gemini,
            None => Mode::Testing,
        };

        if config.mode == Mode::Gemini && api_key.is_none() {
            return Err(ConfigError::MissingApiKey);
        }

        config.gemini_api_key = api_key;

        if let Some(value) = env_value("GEMINI_MODEL") {
            config.gemini_model = value;
        }

        if let Some(value) = env_value("GEMINI_BASE_URL") {
            config.gemini_base_url = value.trim_end_matches('/').to_owned();
        }

        match env_value("GEMINI_THINKING_BUDGET") {
            Some(value) if value.eq_ignore_ascii_case("default") => {
                config.gemini_thinking_budget = None;
            }
            Some(_) => config.gemini_thinking_budget = env_parse("GEMINI_THINKING_BUDGET")?,
            None => {}
        }

        config.bind_addr = bind_address()?;

        if let Some(origins) = allowed_origins()? {
            config.allowed_origins = origins;
        }

        if let Some(millis) = env_parse::<u64>("GEMINI_TIMEOUT_MS")? {
            let millis = between(millis, MIN_TIMEOUT_MS, MAX_TIMEOUT_MS, "GEMINI_TIMEOUT_MS")?;
            config.upstream_timeout = Duration::from_millis(millis);
        }

        if let Some(attempts) = env_parse::<u8>("SOUS_CHEF_MAX_ATTEMPTS")? {
            config.max_attempts = between(
                attempts,
                MIN_ATTEMPTS,
                MAX_ATTEMPTS,
                "SOUS_CHEF_MAX_ATTEMPTS",
            )?;
        }

        if let Some(limit) = env_parse::<u32>("RATE_LIMIT_PER_MINUTE")? {
            config.rate_limit_per_minute = limit;
        }

        Ok(config)
    }
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_parse<T>(name: &'static str) -> Result<Option<T>, ConfigError>
where
    T: FromStr,
    T::Err: Display,
{
    env_value(name)
        .map(|value| {
            value.parse::<T>().map_err(|error| ConfigError::Invalid {
                var: name,
                message: format!("'{value}' is invalid: {error}"),
            })
        })
        .transpose()
}

fn between<T>(value: T, min: T, max: T, var: &'static str) -> Result<T, ConfigError>
where
    T: PartialOrd + Display + Copy,
{
    if value < min || value > max {
        return Err(ConfigError::Invalid {
            var,
            message: format!("must be between {min} and {max}"),
        });
    }

    Ok(value)
}

fn bind_address() -> Result<SocketAddr, ConfigError> {
    if let Some(address) = env_parse("BIND_ADDR")? {
        return Ok(address);
    }

    match env_parse("PORT")? {
        Some(port) => Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)),
        None => Ok(Config::default().bind_addr),
    }
}

fn allowed_origins() -> Result<Option<Vec<HeaderValue>>, ConfigError> {
    let Some(value) = env_value("ALLOWED_ORIGINS") else {
        return Ok(None);
    };

    let origins = value
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            HeaderValue::from_str(origin).map_err(|_| ConfigError::Invalid {
                var: "ALLOWED_ORIGINS",
                message: format!("'{origin}' is not a valid origin header"),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((!origins.is_empty()).then_some(origins))
}
