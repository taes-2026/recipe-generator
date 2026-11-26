use crate::domain::{allowed_units, GenerateRequest, Unit};

pub const SYSTEM_PROMPT: &str = "\
You are the \"GenAI Sous-Chef\" inside SmartPantry, a mobile pantry manager.
Produce exactly one original recipe as JSON that matches the provided response schema.

Hard rules:
1. Respect the household profile: diet, intolerances, restrictions and servings. Never include an ingredient that conflicts with them. If one of the supplied leftovers conflicts with the profile, silently omit it and build the dish from the rest.
2. Cook only with the leftovers and pantry items listed, plus salt, pepper, water and a neutral cooking oil. If you use one of those four, list it in ingredients with optional=true.
3. servings MUST equal the requested number, and every quantity must be scaled to it.
4. ingredient.unit MUST be one of: unit, g, kg, ml, l, pack. Prefer grams/ml for precise amounts, unit for countable pieces.
5. Every step MUST list in \"uses\" the exact names of the ingredients it needs, copied from ingredients[].name. Never reference an ingredient in \"uses\" that is not listed.
6. Set step.timerSeconds only for steps with a timed action (simmer, bake, rest); omit or null otherwise.
7. At most 12 ingredients and 8 steps. Titles under 90 characters. No markdown, no commentary, no extra keys.
8. Write every human-readable text (title, summary, steps, tips) in European Portuguese (pt-PT). Keep ingredient names in Portuguese and reuse the user's names when they are given.
9. Be realistic: prepMinutes and cookMinutes must be plausible for the dish, and the steps must be safe and followable in a home kitchen.";

fn format_amount(quantity: Option<f64>, unit: Option<Unit>) -> String {
    match (quantity, unit) {
        (Some(quantity), Some(unit)) => format!("{quantity} {}", unit.as_str()),
        (Some(quantity), None) => format!("{quantity} unit"),
        (None, Some(unit)) => format!("({})", unit.as_str()),
        (None, None) => "amount unknown".to_owned(),
    }
}

pub fn build_prompt(request: &GenerateRequest, violations: &[String]) -> String {
    let leftovers = request
        .ingredients
        .iter()
        .map(|ingredient| {
            format!(
                "- {}: {}",
                ingredient.name.trim(),
                format_amount(ingredient.quantity, ingredient.unit)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let pantry = if request.pantry.is_empty() {
        "- (nothing else)".to_owned()
    } else {
        request
            .pantry
            .iter()
            .map(|name| format!("- {}", name.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let profile = &request.profile;
    let mut prompt = format!(
        "Household profile\n- servings: {}\n- diet: {}\n- intolerances: {}\n- restrictions: {}\n- maximum total time: {}\n- extra notes: {}\n\nLeftovers to use first:\n{}\n\nOther pantry items:\n{}\n\nAllowed units: {}\n\nBuild one dish for {} people.",
        profile.servings,
        profile.diet,
        list_or_none(&profile.intolerances),
        list_or_none(&profile.restrictions),
        request
            .max_minutes
            .map(|minutes| format!("{minutes} minutes"))
            .unwrap_or_else(|| "no limit".to_owned()),
        request.notes.as_deref().unwrap_or("none"),
        leftovers,
        pantry,
        allowed_units().join(", "),
        profile.servings
    );

    if !violations.is_empty() {
        prompt.push_str("\n\nA previous attempt was rejected by the validator. Fix every problem below and return a corrected recipe:\n");
        for violation in violations {
            prompt.push_str("- ");
            prompt.push_str(violation);
            prompt.push('\n');
        }
    }

    prompt
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProfileConstraints, RequestIngredient};

    fn request() -> GenerateRequest {
        GenerateRequest {
            ingredients: vec![
                RequestIngredient {
                    name: "Tomate".to_owned(),
                    quantity: Some(2.0),
                    unit: Some(crate::domain::Unit::Piece),
                },
                RequestIngredient {
                    name: "Cebola".to_owned(),
                    quantity: None,
                    unit: None,
                },
            ],
            pantry: vec!["Arroz".to_owned()],
            profile: ProfileConstraints {
                servings: 3,
                diet: "vegetarian".to_owned(),
                intolerances: vec!["gluten".to_owned()],
                restrictions: vec![],
            },
            max_minutes: Some(30),
            notes: Some("sem forno".to_owned()),
        }
    }

    #[test]
    fn prompt_carries_every_constraint() {
        let prompt = build_prompt(&request(), &[]);

        assert!(prompt.contains("servings: 3"));
        assert!(prompt.contains("diet: vegetarian"));
        assert!(prompt.contains("intolerances: gluten"));
        assert!(prompt.contains("- Tomate: 2 unit"));
        assert!(prompt.contains("- Arroz"));
        assert!(prompt.contains("30 minutes"));
        assert!(prompt.contains("sem forno"));
    }

    #[test]
    fn prompt_appends_retry_feedback() {
        let prompt = build_prompt(&request(), &["ingredient 'bacon' conflicts".to_owned()]);

        assert!(prompt.contains("rejected by the validator"));
        assert!(prompt.contains("conflicts"));
    }
}
