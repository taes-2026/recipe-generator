#[must_use]
pub fn normalize(input: &str) -> String {
    let mut output = String::with_capacity(input.len());

    for ch in input.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let mapped = match lower {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        };

        output.push(if mapped.is_whitespace() { ' ' } else { mapped });
    }

    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Heuristic: words, close plurals and phrases, not a taxonomy.
#[must_use]
pub fn matches_keyword(haystack: &str, keyword: &str) -> bool {
    let normalized_keyword = normalize(keyword);

    if normalized_keyword.is_empty() {
        return false;
    }

    let normalized_haystack = normalize(haystack);

    if normalized_keyword.contains(' ') {
        return normalized_haystack.contains(&normalized_keyword);
    }

    normalized_haystack
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            token == normalized_keyword
                || (token.starts_with(&normalized_keyword)
                    && token.len() <= normalized_keyword.len() + 2)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_accents_and_casing() {
        assert_eq!(normalize("  Açúcar  Mascavado "), "acucar mascavado");
        assert_eq!(normalize("Sal\tFino"), "sal fino");
    }

    #[test]
    fn matches_plural_and_phrase_keywords() {
        assert!(matches_keyword("Ovos", "ovo"));
        assert!(matches_keyword("farinha de trigo integral", "trigo"));
        assert!(matches_keyword("Leite meio-gordo", "leite"));
        assert!(!matches_keyword("novo", "ovo"));
        assert!(!matches_keyword("melancia", "mel"));
    }
}
