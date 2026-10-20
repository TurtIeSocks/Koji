//! `NameModifier` — the resolved, presence/value-correct name transform driven
//! by [`ApiQueryArgs`]. Built once per request (`From<&ApiQueryArgs>`) and
//! `apply`d to each geofence name during `db::geofence::to_feature` rendering.
//!
//! Relocated from `koji-core::text_utils` (2026-06-16): its only consumer is the
//! db feature-render path, and its `From<&ApiQueryArgs>` builder sets private
//! fields, so it travels with [`ApiQueryArgs`]. The generic, genuinely-shared
//! text helpers (`clean`/`get_mode_acronym`/`separate_by_comma`) stay in
//! koji-core; this only consumes `clean`.

use regex::Regex;

use koji_core::clean;

use crate::query_args::ApiQueryArgs;

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
    parentreplace: Option<String>,
    parentstart: Option<String>,
    parentend: Option<String>,
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
            parentreplace: a.parentreplace.clone(),
            parentstart: a.parentstart.clone(),
            parentend: a.parentend.clone(),
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
    /// Apply the 15-step name transform, byte-for-byte identical to the legacy
    /// free fn (same order, `clean()` on the value-fields, final `trim()` +
    /// empty-guard returning the original string).
    ///
    /// `parent` is the resolved parent name; the three `parent*` steps are
    /// skipped without one.
    pub fn apply(&self, name: &str, parent: Option<&str>) -> String {
        let string = name.to_string();
        let mut mutable = string.clone();
        // Char-based, saturating trims: the offsets arrive from the wire DTO, so
        // raw byte slicing panicked on over-length values (usize underflow for
        // trimend) and on offsets landing mid-multibyte-char (Polish names are
        // the expected domain here — see `unpolish`).
        if let Some(trimstart) = self.trimstart {
            mutable = mutable.chars().skip(trimstart).collect();
        }
        if let Some(trimend) = self.trimend {
            let keep = mutable.chars().count().saturating_sub(trimend);
            mutable = mutable.chars().take(keep).collect();
        }
        if self.alphanumeric {
            mutable = remove_symbols(&mutable);
        }
        if let Some(replacer) = self.replace.as_ref() {
            mutable = mutable.replace(clean(replacer).as_str(), "");
        }
        if let Some(parent) = parent {
            if let Some(replacer) = self.parentreplace.as_ref() {
                mutable = mutable.replacen(parent, clean(replacer).as_str(), 1);
            }
            if let Some(start) = self.parentstart.as_ref() {
                mutable = format!("{parent}{}{mutable}", clean(start));
            }
            if let Some(end) = self.parentend.as_ref() {
                mutable = format!("{mutable}{}{parent}", clean(end));
            }
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

fn remove_symbols(param: &str) -> String {
    let regex = Regex::new(r"[^a-zA-Z0-9\s\-_]").unwrap();
    regex.replace_all(param, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_with(json: serde_json::Value) -> ApiQueryArgs {
        serde_json::from_value(json).unwrap()
    }

    // ── NameModifier presence-triggered bools ────────────────────────────────

    #[test]
    fn name_modifier_presence_triggered_bools() {
        // lowercase: Some(false) STILL lowercases (is_some semantics, parity-critical)
        let nm = NameModifier::from(&args_with(serde_json::json!({ "lowercase": false })));
        assert_eq!(nm.apply("HeLLo", None), "hello");
    }

    #[test]
    fn name_modifier_uppercase_presence_triggered() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "uppercase": false })));
        assert_eq!(nm.apply("hello", None), "HELLO");
    }

    #[test]
    fn name_modifier_alphanumeric_presence_triggered() {
        // alphanumeric: Some(false) still removes symbols.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "alphanumeric": false })));
        assert_eq!(nm.apply("a!b@c#", None), "abc");
    }

    #[test]
    fn name_modifier_capfirst_presence_triggered() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "capfirst": false })));
        assert_eq!(nm.apply("hello world", None), "Hello world");
    }

    #[test]
    fn name_modifier_unpolish_presence_triggered() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": false })));
        assert_eq!(nm.apply("łódź", None), "lodz");
    }

    // ── NameModifier value-triggered transforms ───────────────────────────────

    #[test]
    fn name_modifier_value_triggered_and_order() {
        // trimstart -> alphanumeric -> space-replace, applied in that order.
        let nm = NameModifier::from(&args_with(serde_json::json!({
            "trimstart": 2, "alphanumeric": true, "space": "_"
        })));
        assert_eq!(nm.apply("XX a!b c", None), "_ab_c");
    }

    #[test]
    fn name_modifier_trimend() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimend": 3 })));
        assert_eq!(nm.apply("hello", None), "he");
    }

    #[test]
    fn name_modifier_trims_are_total_on_hostile_offsets_and_multibyte() {
        // Regression: raw byte slicing panicked on all three of these.
        // Over-length trimstart → empty (falls back to original name).
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimstart": 99 })));
        assert_eq!(nm.apply("hello", None), "hello");
        // Over-length trimend → usize underflow before; now empty → fallback.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimend": 99 })));
        assert_eq!(nm.apply("hello", None), "hello");
        // Multibyte: trim counts chars, never splits a codepoint.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimstart": 1 })));
        assert_eq!(nm.apply("łódź", None), "ódź");
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimend": 1 })));
        assert_eq!(nm.apply("łódź", None), "łód");
    }

    #[test]
    fn name_modifier_replace_strip() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "foo" })));
        assert_eq!(nm.apply("prefix-foo-suffix", None), "prefix--suffix");
    }

    #[test]
    fn name_modifier_replace_with_quoted_value() {
        // clean() strips surrounding quotes from the replacer.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "\"foo\"" })));
        assert_eq!(nm.apply("foobar", None), "bar");
    }

    #[test]
    fn name_modifier_capitalize_splits_and_caps() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "capitalize": " " })));
        assert_eq!(nm.apply("hello world", None), "Hello World");
    }

    #[test]
    fn name_modifier_underscore_replace() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "underscore": " " })));
        assert_eq!(nm.apply("hello_world", None), "hello world");
    }

    #[test]
    fn name_modifier_dash_replace() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "dash": "_" })));
        assert_eq!(nm.apply("hello-world", None), "hello_world");
    }

    #[test]
    fn name_modifier_space_replace() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "space": "-" })));
        assert_eq!(nm.apply("hello world", None), "hello-world");
    }

    #[test]
    fn name_modifier_empty_guard_returns_original() {
        // replace strips entire name → empty → guard returns original.
        let nm = NameModifier::from(&args_with(serde_json::json!({ "replace": "short" })));
        assert_eq!(nm.apply("short", None), "short");
    }

    #[test]
    fn name_modifier_trim_whitespace_at_end() {
        // trimend=1 on "x " leaves "x" after trim().
        let nm = NameModifier::from(&args_with(serde_json::json!({ "trimend": 1 })));
        assert_eq!(nm.apply("hello ", None), "hello");
    }

    // ── convert_polish_to_ascii (via NameModifier::apply with unpolish) ──────

    #[test]
    fn unpolish_all_lowercase_polish() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": true })));
        assert_eq!(nm.apply("ąćęłńóśźż", None), "acelnoszz");
    }

    #[test]
    fn unpolish_all_uppercase_polish() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": true })));
        assert_eq!(nm.apply("ĄĆĘŁŃÓŚŹŻ", None), "ACELNOSZZ");
    }

    #[test]
    fn unpolish_mixed_passthrough_non_polish() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "unpolish": true })));
        // ASCII chars pass through unchanged.
        assert_eq!(nm.apply("hello", None), "hello");
    }

    // ── parent-aware transforms ──────────────────────────────────────────────

    #[test]
    fn parentstart_prefixes_the_parent() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "parentstart": "-" })));
        assert_eq!(nm.apply("Centre", Some("London")), "London-Centre");
    }

    #[test]
    fn parentend_suffixes_the_parent() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "parentend": "_" })));
        assert_eq!(nm.apply("Centre", Some("London")), "Centre_London");
    }

    #[test]
    fn parentreplace_swaps_the_first_parent_occurrence() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "parentreplace": "UK" })));
        assert_eq!(
            nm.apply("London London Centre", Some("London")),
            "UK London Centre"
        );
    }

    #[test]
    fn parent_transforms_are_skipped_without_a_parent() {
        let nm = NameModifier::from(&args_with(serde_json::json!({
            "parentstart": "-", "parentend": "-", "parentreplace": "X"
        })));
        assert_eq!(nm.apply("Centre", None), "Centre");
    }

    #[test]
    fn parent_transforms_run_after_replace_and_before_case() {
        // replace strips "!" first, parentreplace then swaps the parent, and
        // uppercase sees the parent text that parentstart added.
        let nm = NameModifier::from(&args_with(serde_json::json!({
            "replace": "!", "parentreplace": "uk", "parentstart": " ", "uppercase": true
        })));
        assert_eq!(nm.apply("a!b", Some("p")), "P AB");
        let nm = NameModifier::from(&args_with(serde_json::json!({
            "parentreplace": "uk", "parentend": "/", "lowercase": true
        })));
        assert_eq!(nm.apply("Paris Centre", Some("Paris")), "uk centre/paris");
    }

    #[test]
    fn parent_args_strip_surrounding_quotes() {
        let nm = NameModifier::from(&args_with(serde_json::json!({ "parentstart": "\" - \"" })));
        assert_eq!(nm.apply("Centre", Some("London")), "London - Centre");
    }
}
