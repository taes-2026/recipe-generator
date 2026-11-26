use super::{recipe::ProfileConstraints, text::matches_keyword};

pub struct Constraint {
    pub label: String,
    pub keywords: Vec<String>,
}

impl Constraint {
    fn new(label: impl Into<String>, keywords: &[&str]) -> Self {
        Self {
            label: label.into(),
            keywords: keywords
                .iter()
                .map(|keyword| (*keyword).to_owned())
                .collect(),
        }
    }
}

const VEGETARIAN: &[&str] = &[
    "carne",
    "frango",
    "galinha",
    "porco",
    "peixe",
    "atum",
    "sardinha",
    "salmao",
    "camarao",
    "bacon",
    "presunto",
    "chourico",
    "salsicha",
    "hamburguer",
    "borrego",
    "peru",
    "anchova",
    "meat",
    "chicken",
    "beef",
    "pork",
    "fish",
    "tuna",
    "salmon",
    "shrimp",
    "prawn",
    "bacon",
    "ham",
    "sausage",
    "turkey",
    "lamb",
    "anchovy",
];

const PESCETARIAN: &[&str] = &[
    "carne",
    "frango",
    "galinha",
    "porco",
    "bacon",
    "presunto",
    "chourico",
    "salsicha",
    "hamburguer",
    "borrego",
    "peru",
    "meat",
    "chicken",
    "beef",
    "pork",
    "bacon",
    "ham",
    "sausage",
    "turkey",
    "lamb",
];

const VEGAN: &[&str] = &[
    "carne",
    "frango",
    "galinha",
    "porco",
    "peixe",
    "atum",
    "sardinha",
    "salmao",
    "camarao",
    "bacon",
    "presunto",
    "chourico",
    "salsicha",
    "hamburguer",
    "borrego",
    "peru",
    "anchova",
    "leite",
    "queijo",
    "manteiga",
    "iogurte",
    "natas",
    "ovo",
    "mel",
    "meat",
    "chicken",
    "beef",
    "pork",
    "fish",
    "tuna",
    "salmon",
    "shrimp",
    "bacon",
    "ham",
    "sausage",
    "milk",
    "cheese",
    "butter",
    "yogurt",
    "cream",
    "egg",
    "honey",
];

const DAIRY: &[&str] = &[
    "leite",
    "queijo",
    "manteiga",
    "iogurte",
    "natas",
    "creme",
    "milk",
    "cheese",
    "butter",
    "yogurt",
    "cream",
    "parmesao",
    "mozarela",
    "requeijao",
];

const GLUTEN: &[&str] = &[
    "trigo",
    "farinha",
    "pao",
    "massa",
    "esparguete",
    "bolacha",
    "centeio",
    "cevada",
    "wheat",
    "flour",
    "bread",
    "pasta",
    "noodle",
    "rye",
    "barley",
    "cracker",
    "couscous",
];

const PEANUT: &[&str] = &["amendoim", "peanut"];
const EGG: &[&str] = &["ovo", "egg", "maionese", "mayonnaise"];
const GRAIN: &[&str] = &[
    "arroz", "trigo", "aveia", "cereal", "centeio", "cevada", "rice", "wheat", "oat", "cereal",
    "barley", "rye",
];
const SEAFOOD: &[&str] = &[
    "peixe", "camarao", "atum", "salmao", "marisco", "bacalhau", "sardinha", "polvo", "lula",
    "fish", "shrimp", "tuna", "salmon", "seafood", "cod", "sardine", "octopus", "squid",
];
const SESAME: &[&str] = &["sesamo", "gergelim", "sesame", "tahini"];
const SHELLFISH: &[&str] = &[
    "camarao",
    "caranguejo",
    "lagosta",
    "mexilhao",
    "ameijoa",
    "shrimp",
    "crab",
    "lobster",
    "prawn",
    "clam",
    "mussel",
    "oyster",
];
const SOY: &[&str] = &["soja", "tofu", "edamame", "soy", "soya"];
const SULFITE: &[&str] = &["sulfito", "vinho", "sulfite", "wine"];
const TREE_NUT: &[&str] = &[
    "amendoa",
    "noz",
    "avela",
    "caju",
    "pistacio",
    "pinhão",
    "pistachio",
    "almond",
    "walnut",
    "hazelnut",
    "cashew",
    "pecan",
];
const WHEAT: &[&str] = &["trigo", "wheat", "farinha", "flour"];
const PORK: &[&str] = &[
    "porco",
    "presunto",
    "bacon",
    "chourico",
    "salsicha",
    "pancetta",
    "prosciutto",
    "pork",
    "ham",
    "sausage",
    "chorizo",
];
const SUGAR: &[&str] = &[
    "acucar", "mel", "xarope", "sugar", "honey", "syrup", "molasses",
];
const BEEF: &[&str] = &[
    "vaca",
    "bife",
    "hamburguer",
    "carne picada",
    "vitela",
    "beef",
    "steak",
    "veal",
    "hamburger",
    "ground beef",
];
const ALCOHOL: &[&str] = &[
    "vinho",
    "cerveja",
    "whisky",
    "vodka",
    "gin",
    "rum",
    "licor",
    "aguardente",
    "wine",
    "beer",
    "vodka",
    "whisky",
    "alcohol",
    "liquor",
];

fn intolerance_keywords(key: &str) -> Option<&'static [&'static str]> {
    match key.to_ascii_lowercase().as_str() {
        "dairy" => Some(DAIRY),
        "gluten" => Some(GLUTEN),
        "peanut" => Some(PEANUT),
        "egg" => Some(EGG),
        "grain" => Some(GRAIN),
        "seafood" => Some(SEAFOOD),
        "sesame" => Some(SESAME),
        "shellfish" => Some(SHELLFISH),
        "soy" => Some(SOY),
        "sulfite" => Some(SULFITE),
        "treenut" => Some(TREE_NUT),
        "wheat" => Some(WHEAT),
        _ => None,
    }
}

fn restriction_keywords(key: &str) -> Option<&'static [&'static str]> {
    match key.to_ascii_lowercase().as_str() {
        "pork" => Some(PORK),
        "sugar" => Some(SUGAR),
        "beef" => Some(BEEF),
        "alcohol" => Some(ALCOHOL),
        _ => None,
    }
}

pub fn constraints(profile: &ProfileConstraints) -> Vec<Constraint> {
    let mut out = Vec::new();

    match profile.diet.to_ascii_lowercase().as_str() {
        "vegetarian" => out.push(Constraint::new("diet:vegetarian", VEGETARIAN)),
        "vegan" => out.push(Constraint::new("diet:vegan", VEGAN)),
        "pescetarian" => out.push(Constraint::new("diet:pescetarian", PESCETARIAN)),
        _ => {}
    }

    for intolerance in &profile.intolerances {
        let label = format!("intolerance:{intolerance}");
        match intolerance_keywords(intolerance) {
            Some(keywords) => out.push(Constraint::new(label, keywords)),
            // Free-text constraints are matched as their own keyword.
            None => out.push(Constraint {
                label: format!("{label}:{intolerance}"),
                keywords: vec![intolerance.to_owned()],
            }),
        }
    }

    for restriction in &profile.restrictions {
        let label = format!("restriction:{restriction}");
        match restriction_keywords(restriction) {
            Some(keywords) => out.push(Constraint::new(label, keywords)),
            None => out.push(Constraint {
                label: format!("{label}:{restriction}"),
                keywords: vec![restriction.to_owned()],
            }),
        }
    }

    out
}

pub fn violations(profile: &ProfileConstraints, ingredient_names: &[String]) -> Vec<String> {
    let mut out = Vec::new();

    for constraint in constraints(profile) {
        for name in ingredient_names {
            if constraint
                .keywords
                .iter()
                .any(|keyword| matches_keyword(name, keyword))
            {
                out.push(format!(
                    "ingredient '{name}' conflicts with {}",
                    constraint.label
                ));
            }
        }
    }

    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(diet: &str, intolerances: &[&str], restrictions: &[&str]) -> ProfileConstraints {
        ProfileConstraints {
            servings: 2,
            diet: diet.to_owned(),
            intolerances: intolerances
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            restrictions: restrictions
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }

    #[test]
    fn flags_meat_for_vegetarian() {
        let violations = violations(&profile("vegetarian", &[], &[]), &["Bacon".to_owned()]);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("diet:vegetarian"));
    }

    #[test]
    fn flags_flour_for_gluten() {
        let violations = violations(
            &profile("omnivore", &["gluten"], &[]),
            &["farinha de trigo".to_owned()],
        );
        assert!(!violations.is_empty());
    }

    #[test]
    fn accepts_clean_ingredients() {
        let violations = violations(
            &profile("vegan", &["gluten"], &["sugar"]),
            &["tomate".to_owned(), "grao-de-bico".to_owned()],
        );
        assert!(violations.is_empty(), "{violations:?}");
    }
}
