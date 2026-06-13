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

pub fn json_related_sort(json: &mut [serde_json::Value], sort_by: &str, order: String) {
    json.sort_by(|a, b| {
        let a = a[sort_by].as_array().unwrap().len();
        let b = b[sort_by].as_array().unwrap().len();
        if order == "asc" { a.cmp(&b) } else { b.cmp(&a) }
    });
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

pub fn name_modifier(string: String, modifiers: &ApiQueryArgs, parent: Option<String>) -> String {
    let mut mutable = string.clone();
    if let Some(trimstart) = modifiers.trimstart {
        mutable = mutable[trimstart..].to_string();
    }
    if let Some(trimend) = modifiers.trimend {
        mutable = mutable[..(mutable.len() - trimend)].to_string();
    }
    if modifiers.alphanumeric.is_some() {
        mutable = remove_symbols(&mutable);
    }
    if let Some(replacer) = modifiers.replace.as_ref() {
        mutable = mutable.replace(clean(replacer).as_str(), "");
    }
    if let Some(parent) = parent {
        if let Some(replacer) = modifiers.parentreplace.as_ref() {
            mutable = mutable.replacen(&parent, clean(replacer).as_str(), 1);
        }
        if let Some(parent_start) = modifiers.parentstart.as_ref() {
            mutable = format!("{}{}{}", parent, clean(parent_start), mutable,);
        }
        if let Some(parent_end) = modifiers.parentend.as_ref() {
            mutable = format!("{}{}{}", mutable, clean(parent_end), parent,);
        }
    }
    if modifiers.lowercase.is_some() {
        mutable = mutable.to_lowercase();
    }
    if modifiers.uppercase.is_some() {
        mutable = mutable.to_uppercase();
    }
    if modifiers.capfirst.is_some() {
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
    if let Some(capitalize) = modifiers.capitalize.as_ref() {
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
    if let Some(replacer) = modifiers.underscore.as_ref() {
        mutable = mutable.replace("_", clean(replacer).as_str());
    }
    if let Some(replacer) = modifiers.dash.as_ref() {
        mutable = mutable.replace("-", clean(replacer).as_str());
    }
    if let Some(replacer) = modifiers.space.as_ref() {
        mutable = mutable.replace(" ", clean(replacer).as_str());
    }
    if modifiers.unpolish.is_some() {
        mutable = convert_polish_to_ascii(mutable);
    }
    mutable = mutable.trim().to_string();
    if mutable.is_empty() {
        log::warn!(
            "Empty string detected for {} {:?}, returning the standard name",
            string,
            modifiers
        );
        string
    } else {
        mutable
    }
}
