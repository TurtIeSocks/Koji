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

    #[test]
    fn name_modifier_presence_triggered_bools() {
        // lowercase: Some(false) STILL lowercases (is_some semantics, parity-critical)
        let nm = NameModifier::from(&args_with(serde_json::json!({ "lowercase": false })));
        assert_eq!(nm.apply("HeLLo"), "hello");
    }

    #[test]
    fn name_modifier_value_triggered_and_order() {
        // trimstart -> alphanumeric -> space-replace, applied in that order.
        // NOTE: corrected vs the plan's asserted "ab_c". Legacy `name_modifier`
        // runs space-replace BEFORE the final trim(), so the leading space (left
        // after trimstart drops "XX") becomes "_", and trim() leaves it (a leading
        // "_" is not whitespace). Verified byte-identical to HEAD's legacy body.
        let nm = NameModifier::from(&args_with(serde_json::json!({
            "trimstart": 2, "alphanumeric": true, "space": "_"
        })));
        assert_eq!(nm.apply("XX a!b c"), "_ab_c"); // " a!b c" -> " ab c" -> "_ab_c"
    }

    #[test]
    fn name_modifier_empty_guard_returns_original() {
        // Empty result -> original string. Reached via a value transform that
        // empties `mutable` (here `replace` strips the whole name), NOT via a
        // trimstart overflow — `mutable[trimstart..]` panics in legacy too when
        // trimstart > len, so the plan's `trimstart: 100` premise was invalid.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "short" })));
        assert_eq!(nm.apply("short"), "short"); // "" -> original
    }
}
