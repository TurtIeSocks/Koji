use regex::Regex;

use crate::ApiQueryArgs;

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

fn convert_polish_to_ascii(param: String) -> String {
    let polish_characters: [(char, char); 18] = [
        ('ą', 'a'),
        ('ć', 'c'),
        ('ę', 'e'),
        ('ł', 'l'),
        ('ń', 'n'),
        ('ó', 'o'),
        ('ś', 's'),
        ('ź', 'z'),
        ('ż', 'z'),
        ('Ą', 'A'),
        ('Ć', 'C'),
        ('Ę', 'E'),
        ('Ł', 'L'),
        ('Ń', 'N'),
        ('Ó', 'O'),
        ('Ś', 'S'),
        ('Ź', 'Z'),
        ('Ż', 'Z'),
    ];

    let mut result = String::new();

    for c in param.chars() {
        let converted_char = polish_characters
            .iter()
            .find_map(|&(polish, ascii)| if c == polish { Some(ascii) } else { None })
            .unwrap_or(c);

        result.push(converted_char);
    }

    result
}

pub fn clean(param: &String) -> String {
    if param.starts_with("\"") && param.ends_with("\"") {
        return param[1..param.len() - 1].to_string();
    }
    param.to_string()
}

fn remove_symbols(param: &str) -> String {
    let regex = Regex::new(r"[^a-zA-Z0-9\s\-_]").unwrap();
    regex.replace_all(param, "").to_string()
}

/// Resolved, presence/value-correct view of the name-manipulation args.
///
/// **Parity:** the five bool fields are *presence*-triggered in the legacy
/// `name_modifier` (`if modifiers.alphanumeric.is_some()` etc.), so `Some(false)`
/// STILL fires the transform. `From<&ApiQueryArgs>` therefore sets them via
/// `.is_some()`, never `.unwrap_or(false)`. The seven `Option<…>` fields stay
/// value-triggered and keep their inner values.
#[derive(Debug, Clone, Default)]
pub struct NameModifier {
    trimstart: Option<usize>,
    trimend: Option<usize>,
    alphanumeric: bool,
    replace: Option<String>,
    lowercase: bool,
    uppercase: bool,
    capfirst: bool,
    capitalize: Option<String>,
    underscore: Option<String>,
    dash: Option<String>,
    space: Option<String>,
    unpolish: bool,
}

impl From<&ApiQueryArgs> for NameModifier {
    fn from(a: &ApiQueryArgs) -> Self {
        Self {
            trimstart: a.trimstart,
            trimend: a.trimend,
            // Presence-triggered: Some(false) still fires. MUST be `.is_some()`.
            alphanumeric: a.alphanumeric.is_some(),
            replace: a.replace.clone(),
            lowercase: a.lowercase.is_some(),
            uppercase: a.uppercase.is_some(),
            capfirst: a.capfirst.is_some(),
            capitalize: a.capitalize.clone(),
            underscore: a.underscore.clone(),
            dash: a.dash.clone(),
            space: a.space.clone(),
            unpolish: a.unpolish.is_some(),
        }
    }
}

impl NameModifier {
    /// Apply the 12-step name transform, byte-for-byte identical to the legacy
    /// free fn (same order, `clean()` on the value-fields, final `trim()` +
    /// empty-guard returning the original string).
    pub fn apply(&self, name: &str) -> String {
        let string = name.to_string();
        let mut mutable = string.clone();
        if let Some(trimstart) = self.trimstart {
            mutable = mutable[trimstart..].to_string();
        }
        if let Some(trimend) = self.trimend {
            mutable = mutable[..(mutable.len() - trimend)].to_string();
        }
        if self.alphanumeric {
            mutable = remove_symbols(&mutable);
        }
        if let Some(replacer) = self.replace.as_ref() {
            mutable = mutable.replace(clean(replacer).as_str(), "");
        }
        if self.lowercase {
            mutable = mutable.to_lowercase();
        }
        if self.uppercase {
            mutable = mutable.to_uppercase();
        }
        if self.capfirst {
            mutable = mutable
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i == 0 {
                        c.to_uppercase().to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect();
        }
        if let Some(capitalize) = self.capitalize.as_ref() {
            mutable = mutable
                .split(clean(capitalize).as_str())
                .map(|word| {
                    word.chars()
                        .enumerate()
                        .map(|(i, c)| {
                            if i == 0 {
                                c.to_uppercase().to_string()
                            } else {
                                c.to_string()
                            }
                        })
                        .collect::<String>()
                })
                .collect::<Vec<String>>()
                .join(clean(capitalize).as_str());
        }
        if let Some(replacer) = self.underscore.as_ref() {
            mutable = mutable.replace("_", clean(replacer).as_str());
        }
        if let Some(replacer) = self.dash.as_ref() {
            mutable = mutable.replace("-", clean(replacer).as_str());
        }
        if let Some(replacer) = self.space.as_ref() {
            mutable = mutable.replace(" ", clean(replacer).as_str());
        }
        if self.unpolish {
            mutable = convert_polish_to_ascii(mutable);
        }
        mutable = mutable.trim().to_string();
        if mutable.is_empty() {
            log::warn!(
                "Empty string detected for {} {:?}, returning the standard name",
                string,
                self
            );
            string
        } else {
            mutable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApiQueryArgs;

    fn args_with(json: serde_json::Value) -> ApiQueryArgs {
        serde_json::from_value(json).unwrap()
    }

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

    // ── NameModifier presence-triggered bools ────────────────────────────────

    #[test]
    fn name_modifier_presence_triggered_bools() {
        // lowercase: Some(false) STILL lowercases (is_some semantics, parity-critical)
        let nm = NameModifier::from(&args_with(serde_json::json!({ "lowercase": false })));
        assert_eq!(nm.apply("HeLLo"), "hello");
    }

    #[test]
    fn name_modifier_uppercase_presence_triggered() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "uppercase": false })));
        assert_eq!(nm.apply("hello"), "HELLO");
    }

    #[test]
    fn name_modifier_alphanumeric_presence_triggered() {
        // alphanumeric: Some(false) still removes symbols.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "alphanumeric": false })));
        assert_eq!(nm.apply("a!b@c#"), "abc");
    }

    #[test]
    fn name_modifier_capfirst_presence_triggered() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "capfirst": false })));
        assert_eq!(nm.apply("hello world"), "Hello world");
    }

    #[test]
    fn name_modifier_unpolish_presence_triggered() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": false })));
        assert_eq!(nm.apply("łódź"), "lodz");
    }

    // ── NameModifier value-triggered transforms ───────────────────────────────

    #[test]
    fn name_modifier_value_triggered_and_order() {
        // trimstart -> alphanumeric -> space-replace, applied in that order.
        let nm = NameModifier::from(&args_with(serde_json::json!({
            "trimstart": 2, "alphanumeric": true, "space": "_"
        })));
        assert_eq!(nm.apply("XX a!b c"), "_ab_c");
    }

    #[test]
    fn name_modifier_trimend() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimend": 3 })));
        assert_eq!(nm.apply("hello"), "he");
    }

    #[test]
    fn name_modifier_replace_strip() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "foo" })));
        assert_eq!(nm.apply("prefix-foo-suffix"), "prefix--suffix");
    }

    #[test]
    fn name_modifier_replace_with_quoted_value() {
        // clean() strips surrounding quotes from the replacer.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "\"foo\"" })));
        assert_eq!(nm.apply("foobar"), "bar");
    }

    #[test]
    fn name_modifier_capitalize_splits_and_caps() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "capitalize": " " })));
        assert_eq!(nm.apply("hello world"), "Hello World");
    }

    #[test]
    fn name_modifier_underscore_replace() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "underscore": " " })));
        assert_eq!(nm.apply("hello_world"), "hello world");
    }

    #[test]
    fn name_modifier_dash_replace() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "dash": "_" })));
        assert_eq!(nm.apply("hello-world"), "hello_world");
    }

    #[test]
    fn name_modifier_space_replace() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "space": "-" })));
        assert_eq!(nm.apply("hello world"), "hello-world");
    }

    #[test]
    fn name_modifier_empty_guard_returns_original() {
        // replace strips entire name → empty → guard returns original.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "short" })));
        assert_eq!(nm.apply("short"), "short");
    }

    #[test]
    fn name_modifier_trim_whitespace_at_end() {
        // trimend=1 on "x " leaves "x" after trim().
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimend": 1 })));
        assert_eq!(nm.apply("hello "), "hello");
    }

    // ── convert_polish_to_ascii (via NameModifier::apply with unpolish) ──────

    #[test]
    fn unpolish_all_lowercase_polish() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": true })));
        assert_eq!(nm.apply("ąćęłńóśźż"), "acelnoszz");
    }

    #[test]
    fn unpolish_all_uppercase_polish() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": true })));
        assert_eq!(nm.apply("ĄĆĘŁŃÓŚŹŻ"), "ACELNOSZZ");
    }

    #[test]
    fn unpolish_mixed_passthrough_non_polish() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": true })));
        // ASCII chars pass through unchanged.
        assert_eq!(nm.apply("hello"), "hello");
    }
}
