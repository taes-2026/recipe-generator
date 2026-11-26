use serde::{Deserialize, Serialize};

/// Mirrors UNITS in ../code/src/domain/items.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    #[serde(rename = "unit")]
    Piece,
    Kg,
    G,
    L,
    Ml,
    Pack,
}

impl Unit {
    pub const ALL: [Unit; 6] = [
        Unit::Piece,
        Unit::Kg,
        Unit::G,
        Unit::L,
        Unit::Ml,
        Unit::Pack,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Piece => "unit",
            Self::Kg => "kg",
            Self::G => "g",
            Self::L => "l",
            Self::Ml => "ml",
            Self::Pack => "pack",
        }
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

pub fn allowed_units() -> Vec<&'static str> {
    Unit::ALL.iter().map(|unit| unit.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_keep_the_flat_wire_format() {
        assert_eq!(
            Unit::ALL.map(Unit::as_str),
            ["unit", "kg", "g", "l", "ml", "pack"]
        );
        assert_eq!(
            serde_json::to_string(&Unit::Piece).expect("unit serialises"),
            "\"unit\""
        );
        assert!(serde_json::from_str::<Unit>("\"colher\"").is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub ingredients: Vec<RequestIngredient>,
    #[serde(default)]
    pub pantry: Vec<String>,
    pub profile: ProfileConstraints,
    #[serde(default)]
    pub max_minutes: Option<u32>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestIngredient {
    pub name: String,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub unit: Option<Unit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileConstraints {
    pub servings: u8,
    #[serde(default = "default_diet")]
    pub diet: String,
    #[serde(default)]
    pub intolerances: Vec<String>,
    #[serde(default)]
    pub restrictions: Vec<String>,
}

fn default_diet() -> String {
    "omnivore".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedRecipe {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub servings: u8,
    pub prep_minutes: u32,
    pub cook_minutes: u32,
    pub ingredients: Vec<GeneratedIngredient>,
    pub steps: Vec<GeneratedStep>,
    #[serde(default)]
    pub tips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedIngredient {
    pub name: String,
    pub quantity: f64,
    pub unit: Unit,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    // Recomputed server-side; never taken from the model.
    pub from_pantry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedStep {
    pub n: u32,
    pub text: String,
    #[serde(default)]
    // Exact ingredient names; Cook Mode highlights only these.
    pub uses: Vec<String>,
    #[serde(default)]
    pub timer_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResponse {
    pub id: String,
    pub recipe: GeneratedRecipe,
    pub meta: ResponseMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMeta {
    pub source: String,
    pub model: String,
    pub attempts: u8,
    pub warnings: Vec<String>,
}
