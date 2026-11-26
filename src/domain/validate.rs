use std::collections::HashSet;

use super::{
    constraints,
    recipe::{GenerateRequest, GeneratedRecipe},
    text::normalize,
};

pub const MAX_LEFTOVERS: usize = 20;
pub const MAX_PANTRY: usize = 40;
pub const MAX_RECIPE_INGREDIENTS: usize = 15;
pub const MAX_STEPS: usize = 15;

const MIN_SERVINGS: u8 = 1;
const MAX_SERVINGS: u8 = 12;
const MIN_MAX_MINUTES: u32 = 5;
const MAX_MAX_MINUTES: u32 = 240;
const MAX_NAME_CHARS: usize = 80;
const MAX_TITLE_CHARS: usize = 100;
const MAX_SUMMARY_CHARS: usize = 600;
const MAX_STEP_CHARS: usize = 1_200;
const MAX_NOTES_CHARS: usize = 500;
const MAX_TIMER_SECONDS: u32 = 7_200;
const MAX_MINUTES_PER_PHASE: u32 = 1_440;
const MAX_QUANTITY: f64 = 100_000.0;

fn is_positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value <= MAX_QUANTITY
}

fn fail_on(violations: Vec<String>) -> Result<(), Vec<String>> {
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

pub fn validate_request(request: &GenerateRequest) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();

    if request.ingredients.is_empty() {
        violations.push("at least one leftover ingredient is required".to_owned());
    }

    if request.ingredients.len() > MAX_LEFTOVERS {
        violations.push(format!(
            "at most {MAX_LEFTOVERS} leftover ingredients are accepted, got {}",
            request.ingredients.len()
        ));
    }

    if request.pantry.len() > MAX_PANTRY {
        violations.push(format!("at most {MAX_PANTRY} pantry items are accepted"));
    }

    let mut seen = HashSet::new();

    for (index, ingredient) in request.ingredients.iter().enumerate() {
        let name = ingredient.name.trim();

        if name.is_empty() {
            violations.push(format!("ingredient #{} has an empty name", index + 1));
        } else if name.chars().count() > MAX_NAME_CHARS {
            violations.push(format!(
                "ingredient '{name}' is longer than {MAX_NAME_CHARS} characters"
            ));
        } else if !seen.insert(normalize(name)) {
            violations.push(format!("ingredient '{name}' is duplicated"));
        }

        if let Some(quantity) = ingredient.quantity {
            if !is_positive_finite(quantity) {
                violations.push(format!("ingredient '{name}' has an invalid quantity"));
            }
        }
    }

    if !(MIN_SERVINGS..=MAX_SERVINGS).contains(&request.profile.servings) {
        violations.push(format!(
            "servings must be between {MIN_SERVINGS} and {MAX_SERVINGS}"
        ));
    }

    if let Some(max_minutes) = request.max_minutes {
        if !(MIN_MAX_MINUTES..=MAX_MAX_MINUTES).contains(&max_minutes) {
            violations.push(format!(
                "maxMinutes must be between {MIN_MAX_MINUTES} and {MAX_MAX_MINUTES}"
            ));
        }
    }

    if let Some(notes) = &request.notes {
        if notes.chars().count() > MAX_NOTES_CHARS {
            violations.push(format!(
                "notes are longer than {MAX_NOTES_CHARS} characters"
            ));
        }
    }

    fail_on(violations)
}

pub fn validate_recipe(
    recipe: &GeneratedRecipe,
    request: &GenerateRequest,
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();

    validate_recipe_metadata(recipe, request, &mut violations);
    let ingredient_names = validate_recipe_ingredients(recipe, &mut violations);
    validate_recipe_steps(recipe, &ingredient_names, &mut violations);

    let names: Vec<String> = recipe
        .ingredients
        .iter()
        .map(|ingredient| ingredient.name.clone())
        .collect();

    violations.extend(constraints::violations(&request.profile, &names));

    fail_on(violations)
}

fn validate_recipe_metadata(
    recipe: &GeneratedRecipe,
    request: &GenerateRequest,
    violations: &mut Vec<String>,
) {
    let title = recipe.title.trim();

    if title.is_empty() {
        violations.push("title is empty".to_owned());
    } else if title.chars().count() > MAX_TITLE_CHARS {
        violations.push(format!("title is longer than {MAX_TITLE_CHARS} characters"));
    }

    if recipe.servings != request.profile.servings {
        violations.push(format!(
            "recipe serves {} but the profile asked for {}",
            recipe.servings, request.profile.servings
        ));
    }

    if recipe.summary.chars().count() > MAX_SUMMARY_CHARS {
        violations.push(format!(
            "summary is longer than {MAX_SUMMARY_CHARS} characters"
        ));
    }

    if recipe.prep_minutes > MAX_MINUTES_PER_PHASE || recipe.cook_minutes > MAX_MINUTES_PER_PHASE {
        violations.push("prep/cook minutes are implausible".to_owned());
    }
}

fn validate_recipe_ingredients(
    recipe: &GeneratedRecipe,
    violations: &mut Vec<String>,
) -> HashSet<String> {
    if recipe.ingredients.is_empty() {
        violations.push("the recipe has no ingredients".to_owned());
    } else if recipe.ingredients.len() > MAX_RECIPE_INGREDIENTS {
        violations.push(format!(
            "the recipe has {} ingredients, at most {MAX_RECIPE_INGREDIENTS} are allowed",
            recipe.ingredients.len()
        ));
    }

    let mut names = HashSet::new();
    let mut seen = HashSet::new();

    for ingredient in &recipe.ingredients {
        let name = ingredient.name.trim();

        if name.is_empty() {
            violations.push("the recipe contains an ingredient with an empty name".to_owned());
            continue;
        }

        if name.chars().count() > MAX_NAME_CHARS {
            violations.push(format!(
                "ingredient '{name}' is longer than {MAX_NAME_CHARS} characters"
            ));
        }

        if !seen.insert(normalize(name)) {
            violations.push(format!("ingredient '{name}' appears twice"));
        }

        names.insert(normalize(name));

        if !is_positive_finite(ingredient.quantity) {
            violations.push(format!("ingredient '{name}' has an invalid quantity"));
        }
    }

    names
}

fn validate_recipe_steps(
    recipe: &GeneratedRecipe,
    ingredient_names: &HashSet<String>,
    violations: &mut Vec<String>,
) {
    if recipe.steps.is_empty() {
        violations.push("the recipe has no steps".to_owned());
    } else if recipe.steps.len() > MAX_STEPS {
        violations.push(format!(
            "the recipe has {} steps, at most {MAX_STEPS} are allowed",
            recipe.steps.len()
        ));
    }

    for (index, step) in recipe.steps.iter().enumerate() {
        if step.n as usize != index + 1 {
            violations.push(format!("step {} is misnumbered as {}", index + 1, step.n));
        }

        if step.text.trim().is_empty() {
            violations.push(format!("step {} has no text", step.n));
        } else if step.text.chars().count() > MAX_STEP_CHARS {
            violations.push(format!(
                "step {} is longer than {MAX_STEP_CHARS} characters",
                step.n
            ));
        }

        if let Some(timer) = step.timer_seconds {
            if timer > MAX_TIMER_SECONDS {
                violations.push(format!("step {} has an implausible timer", step.n));
            }
        }

        for used in &step.uses {
            if !ingredient_names.contains(&normalize(used)) {
                violations.push(format!(
                    "step {} uses '{used}', which is not one of ingredients[].name",
                    step.n
                ));
            }
        }
    }
}
