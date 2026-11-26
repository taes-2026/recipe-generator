use crate::{
    config::{Config, ConfigError, Mode},
    domain::{GenerateRequest, GeneratedRecipe},
};

use super::{gemini::GeminiProvider, testing::TestingProvider};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider timed out")]
    Timeout,
    #[error("provider rate limited")]
    RateLimited,
    #[error("provider error: {0}")]
    Upstream(String),
    #[error("provider refused: {0}")]
    Refused(String),
    #[error("provider returned invalid output: {0}")]
    InvalidOutput(String),
    /// Recoverable: one retry with the parser message fed back to the model.
    #[error("provider returned invalid schema: {0}")]
    InvalidSchema(String),
}

impl ProviderError {
    // Timeouts are not retried: the user is already waiting.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::RateLimited | Self::Upstream(_))
    }
}

pub enum Provider {
    Gemini(GeminiProvider),
    Testing(TestingProvider),
}

impl Provider {
    pub fn from_config(config: &Config) -> Result<Self, ConfigError> {
        match config.mode {
            Mode::Gemini => {
                let api_key = config
                    .gemini_api_key
                    .clone()
                    .ok_or(ConfigError::MissingApiKey)?;
                Ok(Self::Gemini(GeminiProvider::new(config, api_key)?))
            }
            Mode::Testing => Ok(Self::Testing(TestingProvider)),
        }
    }

    pub async fn generate(
        &self,
        prompt: &str,
        request: &GenerateRequest,
    ) -> Result<GeneratedRecipe, ProviderError> {
        match self {
            Self::Gemini(provider) => provider.generate(prompt).await,
            Self::Testing(provider) => provider.generate(request),
        }
    }

    pub fn source(&self) -> &'static str {
        match self {
            Self::Gemini(_) => "gemini",
            Self::Testing(_) => "testing",
        }
    }

    pub fn model(&self) -> &str {
        match self {
            Self::Gemini(provider) => provider.model(),
            Self::Testing(_) => "testing-v1",
        }
    }
}
