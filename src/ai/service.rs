use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    domain::{
        text::normalize,
        validate::{validate_recipe, validate_request},
        GenerateRequest, GenerateResponse, GeneratedRecipe, ResponseMeta,
    },
    error::ApiError,
};

use super::{
    prompt,
    provider::{Provider, ProviderError},
};

pub struct SousChefService {
    provider: Provider,
    max_attempts: u8,
}

impl SousChefService {
    #[must_use = "the service must be built before it can generate"]
    pub fn new(provider: Provider, max_attempts: u8) -> Self {
        Self {
            provider,
            max_attempts: max_attempts.max(1),
        }
    }

    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, ApiError> {
        validate_request(&request).map_err(ApiError::InvalidInput)?;

        let mut feedback: Vec<String> = Vec::new();
        let mut last_failure: Option<ApiError> = None;

        for attempt in 1..=self.max_attempts {
            let built_prompt = prompt::build_prompt(&request, &feedback);

            match self.provider.generate(&built_prompt, &request).await {
                Ok(mut recipe) => {
                    apply_pantry_flags(&mut recipe, &request);

                    match validate_recipe(&recipe, &request) {
                        Ok(()) => {
                            return Ok(GenerateResponse {
                                id: format!("ai-{}", Uuid::new_v4()),
                                recipe,
                                meta: ResponseMeta {
                                    source: self.provider.source().to_owned(),
                                    model: self.provider.model().to_owned(),
                                    attempts: attempt,
                                    warnings: if attempt > 1 {
                                        vec![format!("recipe accepted after {attempt} attempts")]
                                    } else {
                                        Vec::new()
                                    },
                                },
                            });
                        }
                        Err(violations) => {
                            tracing::warn!(attempt, ?violations, "model output rejected");
                            last_failure = Some(ApiError::InvalidModelOutput(violations.clone()));
                            feedback = violations;
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(attempt, error = %error, "provider call failed");

                    match error {
                        // Schema slips are recoverable with the parser message.
                        ProviderError::InvalidSchema(message) if attempt < self.max_attempts => {
                            feedback = vec![format!(
                                "the previous answer did not match the JSON schema: {message}"
                            )];
                            last_failure = Some(ApiError::InvalidModelOutput(vec![message]));
                            continue;
                        }
                        error if error.retryable() && attempt < self.max_attempts => {
                            last_failure = Some(api_error_from(error));
                            // Capacity errors usually clear in a second or two.
                            tokio::time::sleep(std::time::Duration::from_millis(
                                750 * u64::from(attempt),
                            ))
                            .await;
                            continue;
                        }
                        error => return Err(api_error_from(error)),
                    }
                }
            }
        }

        Err(last_failure.unwrap_or_else(|| ApiError::Internal("no attempt was made".to_owned())))
    }
}

// Never trust fromPantry from the model: derive it from the request instead.
fn apply_pantry_flags(recipe: &mut GeneratedRecipe, request: &GenerateRequest) {
    let known: HashSet<String> = request
        .ingredients
        .iter()
        .map(|ingredient| normalize(&ingredient.name))
        .chain(request.pantry.iter().map(|name| normalize(name)))
        .collect();

    for ingredient in &mut recipe.ingredients {
        ingredient.name = ingredient.name.trim().to_owned();
        ingredient.from_pantry = known.contains(&normalize(&ingredient.name));
    }
}

fn api_error_from(error: ProviderError) -> ApiError {
    match error {
        ProviderError::Timeout => ApiError::UpstreamTimeout,
        ProviderError::RateLimited => ApiError::UpstreamRateLimited,
        ProviderError::Upstream(message) => ApiError::Upstream(message),
        ProviderError::Refused(message) => ApiError::UpstreamRefused(message),
        ProviderError::InvalidOutput(message) | ProviderError::InvalidSchema(message) => {
            ApiError::InvalidModelOutput(vec![message])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProfileConstraints, RequestIngredient};

    fn request() -> GenerateRequest {
        GenerateRequest {
            ingredients: vec![RequestIngredient {
                name: " Tomate ".to_owned(),
                quantity: Some(2.0),
                unit: Some(crate::domain::Unit::Piece),
            }],
            pantry: vec!["Arroz".to_owned()],
            profile: ProfileConstraints {
                servings: 2,
                diet: "omnivore".to_owned(),
                intolerances: vec![],
                restrictions: vec![],
            },
            max_minutes: None,
            notes: None,
        }
    }

    #[test]
    fn pantry_flags_are_recomputed_from_the_request() {
        let request = request();
        let mut recipe = GeneratedRecipe {
            title: "Teste".to_owned(),
            summary: String::new(),
            servings: 2,
            prep_minutes: 5,
            cook_minutes: 5,
            ingredients: vec![
                crate::domain::GeneratedIngredient {
                    name: "Tomate".to_owned(),
                    quantity: 1.0,
                    unit: crate::domain::Unit::Piece,
                    optional: false,
                    from_pantry: false,
                },
                crate::domain::GeneratedIngredient {
                    name: "Queijo".to_owned(),
                    quantity: 50.0,
                    unit: crate::domain::Unit::G,
                    optional: false,
                    from_pantry: true,
                },
            ],
            steps: vec![],
            tips: vec![],
        };

        apply_pantry_flags(&mut recipe, &request);

        assert!(recipe.ingredients[0].from_pantry);
        assert!(!recipe.ingredients[1].from_pantry);
    }
}
