pub fn get_mode_acronym(instance_type: Option<&String>) -> String {
    match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" | "auto_quest" => "AQ",
            "CirclePokemon" | "circle_pokemon" => "CP",
            "CircleSmartPokemon" | "circle_smart_pokemon" => "CSP",
            "CircleRaid" | "circle_raid" => "CR",
            "CircleSmartRaid" | "circle_smart_raid" => "CSR",
            "PokemonIv" | "pokemon_iv" => "IV",
            "Leveling" | "leveling" => "L",
            "CircleQuest" | "circle_quest" => "CQ",
            "AutoTth" | "auto_tth" => "ATTH",
            "AutoPokemon" | "auto_pokemon" => "AP",
            _ => "U",
        },
        None => "U",
    }
    .to_string()
}

pub fn separate_by_comma(param: &Option<String>) -> Vec<String> {
    if let Some(param) = param {
        param.split(",").map(|x| x.to_string()).collect()
    } else {
        vec![]
    }
}

pub fn clean(param: &String) -> String {
    if param.starts_with("\"") && param.ends_with("\"") {
        return param[1..param.len() - 1].to_string();
    }
    param.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── get_mode_acronym ─────────────────────────────────────────────────────

    #[test]
    fn get_mode_acronym_known_variants() {
        let cases = [
            ("AutoQuest", "AQ"),
            ("auto_quest", "AQ"),
            ("CirclePokemon", "CP"),
            ("circle_pokemon", "CP"),
            ("CircleSmartPokemon", "CSP"),
            ("circle_smart_pokemon", "CSP"),
            ("CircleRaid", "CR"),
            ("circle_raid", "CR"),
            ("CircleSmartRaid", "CSR"),
            ("circle_smart_raid", "CSR"),
            ("PokemonIv", "IV"),
            ("pokemon_iv", "IV"),
            ("Leveling", "L"),
            ("leveling", "L"),
            ("CircleQuest", "CQ"),
            ("circle_quest", "CQ"),
            ("AutoTth", "ATTH"),
            ("auto_tth", "ATTH"),
            ("AutoPokemon", "AP"),
            ("auto_pokemon", "AP"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                get_mode_acronym(Some(&input.to_string())),
                expected,
                "failed for {input}"
            );
        }
    }

    #[test]
    fn get_mode_acronym_unknown_returns_u() {
        assert_eq!(get_mode_acronym(Some(&"bogus".to_string())), "U");
    }

    #[test]
    fn get_mode_acronym_none_returns_u() {
        assert_eq!(get_mode_acronym(None), "U");
    }

    // ── separate_by_comma ────────────────────────────────────────────────────

    #[test]
    fn separate_by_comma_splits_on_comma() {
        let v = separate_by_comma(&Some("a,b,c".to_string()));
        assert_eq!(v, vec!["a", "b", "c"]);
    }

    #[test]
    fn separate_by_comma_single_item() {
        let v = separate_by_comma(&Some("only".to_string()));
        assert_eq!(v, vec!["only"]);
    }

    #[test]
    fn separate_by_comma_none_returns_empty() {
        let v = separate_by_comma(&None);
        assert!(v.is_empty());
    }

    #[test]
    fn separate_by_comma_empty_string_gives_one_empty_element() {
        // "".split(",") yields one empty string.
        let v = separate_by_comma(&Some(String::new()));
        assert_eq!(v, vec![""]);
    }

    // ── clean ────────────────────────────────────────────────────────────────

    #[test]
    fn clean_strips_surrounding_quotes() {
        assert_eq!(clean(&"\"hello\"".to_string()), "hello");
    }

    #[test]
    fn clean_passthrough_when_not_quoted() {
        assert_eq!(clean(&"hello".to_string()), "hello");
    }

    #[test]
    fn clean_single_char_quoted() {
        assert_eq!(clean(&"\"x\"".to_string()), "x");
    }
}
