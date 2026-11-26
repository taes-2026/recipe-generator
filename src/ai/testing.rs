use crate::domain::{
    constraints, text::normalize, GenerateRequest, GeneratedIngredient, GeneratedRecipe,
    GeneratedStep, ProfileConstraints, Unit,
};

use super::provider::ProviderError;

pub struct TestingProvider;

impl TestingProvider {
    pub fn generate(&self, request: &GenerateRequest) -> Result<GeneratedRecipe, ProviderError> {
        let base = base_ingredients(request)?;
        let names: Vec<String> = base.iter().map(|(name, _, _)| name.clone()).collect();
        let has_oil = names.iter().any(|name| normalize(name) == "azeite");
        let has_salt = names.iter().any(|name| normalize(name) == "sal");

        Ok(GeneratedRecipe {
            title: testing_title(&names),
            summary: "Receita gerada localmente em modo de testes.".to_owned(),
            servings: request.profile.servings,
            prep_minutes: 10,
            cook_minutes: 15,
            ingredients: testing_ingredients(base, has_oil, has_salt),
            steps: testing_steps(&names),
            tips: vec![
                "Modo de testes: substitui a chamada ao Gemini em testes e demonstrações offline."
                    .to_owned(),
            ],
        })
    }
}

fn is_compatible(profile: &ProfileConstraints, name: &str) -> bool {
    constraints::violations(profile, &[name.to_owned()]).is_empty()
}

fn base_ingredients(request: &GenerateRequest) -> Result<Vec<(String, f64, Unit)>, ProviderError> {
    let leftovers: Vec<_> = request
        .ingredients
        .iter()
        .filter(|ingredient| is_compatible(&request.profile, &ingredient.name))
        .take(6)
        .map(|ingredient| {
            (
                ingredient.name.trim().to_owned(),
                ingredient.quantity.unwrap_or(1.0).max(0.5),
                ingredient.unit.unwrap_or(Unit::Piece),
            )
        })
        .collect();

    if !leftovers.is_empty() {
        return Ok(leftovers);
    }

    let pantry: Vec<_> = request
        .pantry
        .iter()
        .filter(|name| is_compatible(&request.profile, name.as_str()))
        .take(3)
        .map(|name| (name.trim().to_owned(), 1.0, Unit::Piece))
        .collect();

    if pantry.is_empty() {
        return Err(ProviderError::InvalidOutput(
            "every supplied ingredient conflicts with the profile constraints".to_owned(),
        ));
    }

    Ok(pantry)
}

fn testing_title(names: &[String]) -> String {
    match names.len() {
        1 => format!("Salteado de {}", names[0]),
        2 => format!("Salteado de {} e {}", names[0], names[1]),
        _ => format!("Salteado de {}, {} e {}", names[0], names[1], names[2]),
    }
}

fn testing_ingredients(
    base: Vec<(String, f64, Unit)>,
    has_oil: bool,
    has_salt: bool,
) -> Vec<GeneratedIngredient> {
    let mut ingredients: Vec<GeneratedIngredient> = base
        .into_iter()
        .map(|(name, quantity, unit)| GeneratedIngredient {
            name,
            quantity,
            unit,
            optional: false,
            from_pantry: true,
        })
        .collect();

    for (name, quantity, unit) in [("Azeite", 20.0, Unit::Ml), ("Sal", 2.0, Unit::G)] {
        let present = if name == "Azeite" { has_oil } else { has_salt };

        if !present {
            ingredients.push(GeneratedIngredient {
                name: name.to_owned(),
                quantity,
                unit,
                optional: true,
                from_pantry: false,
            });
        }
    }

    ingredients
}

fn testing_steps(names: &[String]) -> Vec<GeneratedStep> {
    let rest: Vec<String> = names.iter().skip(1).cloned().collect();
    let mut uses = rest.clone();
    uses.push("Sal".to_owned());

    vec![
        GeneratedStep {
            n: 1,
            text: format!(
                "Prepare os ingredientes: {}. Corte tudo em pedaços regulares.",
                names.join(", ")
            ),
            uses: names.to_vec(),
            timer_seconds: None,
        },
        GeneratedStep {
            n: 2,
            text: format!(
                "Aqueça o azeite numa frigideira e salteie {} durante 6 minutos, mexendo de vez em quando.",
                names[0]
            ),
            uses: vec![names[0].clone(), "Azeite".to_owned()],
            timer_seconds: Some(360),
        },
        GeneratedStep {
            n: 3,
            text: if rest.is_empty() {
                "Deixe apurar mais 5 minutos, tempere com sal e sirva.".to_owned()
            } else {
                format!(
                    "Junte {} e deixe cozinhar 5 minutos. Tempere com sal e sirva.",
                    rest.join(" e ")
                )
            },
            uses,
            timer_seconds: Some(300),
        },
    ]
}
