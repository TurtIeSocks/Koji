//! Integration tests for the `StrEnum` derive. Proc-macro crates can't host
//! `#[test]` in `src/`, so the macro's behavioral spec lives here.

use macros::StrEnum;

// ---------------------------------------------------------------------------
// No-default enum: must keep the original contract (const as_str, unknown→None).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
enum Plain {
    #[str("alpha")]
    Alpha,
    #[str("beta")]
    Beta,
}

#[test]
fn plain_as_str_is_const_and_canonical() {
    // const-callable: if this compiles in a const context, as_str is const.
    const A: &str = Plain::Alpha.as_str();
    assert_eq!(A, "alpha");
    assert_eq!(Plain::Beta.as_str(), "beta");
}

#[test]
fn plain_from_str_unknown_is_none() {
    assert_eq!(Plain::from_str_opt("alpha"), Some(Plain::Alpha));
    assert_eq!(Plain::from_str_opt("nope"), None);
}

#[test]
fn plain_case_insensitive() {
    assert_eq!(Plain::from_str_opt("ALPHA"), Some(Plain::Alpha));
    assert_eq!(Plain::from_str_opt("Beta"), Some(Plain::Beta));
}

#[test]
fn plain_deserialize_unknown_errors() {
    assert!(serde_json::from_str::<Plain>("\"nope\"").is_err());
    let a: Plain = serde_json::from_str("\"ALPHA\"").unwrap();
    assert_eq!(a, Plain::Alpha);
}

// ---------------------------------------------------------------------------
// Default catch-all + aliases.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
enum Fancy {
    #[str("unset", alias("", "none"))]
    Unset,
    #[str("pointcount", alias("cluster_count", "point_count"))]
    PointCount,
    #[str("s2cell", alias("s2"))]
    S2Cell,
    #[str(default)]
    Custom(String),
}

#[test]
fn fancy_canonical_roundtrip() {
    assert_eq!(Fancy::from_str_opt("unset"), Some(Fancy::Unset));
    assert_eq!(Fancy::from_str_opt("pointcount"), Some(Fancy::PointCount));
    assert_eq!(Fancy::from_str_opt("s2cell"), Some(Fancy::S2Cell));
}

#[test]
fn fancy_aliases_resolve_to_variant() {
    assert_eq!(Fancy::from_str_opt(""), Some(Fancy::Unset));
    assert_eq!(Fancy::from_str_opt("none"), Some(Fancy::Unset));
    assert_eq!(
        Fancy::from_str_opt("cluster_count"),
        Some(Fancy::PointCount)
    );
    assert_eq!(Fancy::from_str_opt("point_count"), Some(Fancy::PointCount));
    assert_eq!(Fancy::from_str_opt("s2"), Some(Fancy::S2Cell));
}

#[test]
fn fancy_aliases_are_case_insensitive() {
    assert_eq!(Fancy::from_str_opt("NONE"), Some(Fancy::Unset));
    assert_eq!(
        Fancy::from_str_opt("Cluster_Count"),
        Some(Fancy::PointCount)
    );
    assert_eq!(Fancy::from_str_opt("S2"), Some(Fancy::S2Cell));
}

#[test]
fn fancy_unknown_becomes_custom_with_original_case() {
    // Default catch-all: from_str_opt always returns Some, preserving ORIGINAL case.
    assert_eq!(
        Fancy::from_str_opt("WeirdThing"),
        Some(Fancy::Custom("WeirdThing".to_string()))
    );
    assert_eq!(
        Fancy::from_str_opt("tsp"),
        Some(Fancy::Custom("tsp".to_string()))
    );
}

#[test]
fn fancy_as_str_follows_variant_and_inner() {
    assert_eq!(Fancy::Unset.as_str(), "unset");
    assert_eq!(Fancy::PointCount.as_str(), "pointcount");
    assert_eq!(Fancy::S2Cell.as_str(), "s2cell");
    // Custom returns its inner string verbatim.
    let c = Fancy::Custom("MixedCase".to_string());
    assert_eq!(c.as_str(), "MixedCase");
}

#[test]
fn fancy_display_follows_as_str() {
    assert_eq!(Fancy::PointCount.to_string(), "pointcount");
    assert_eq!(Fancy::Custom("xyz".to_string()).to_string(), "xyz");
}

#[test]
fn fancy_serialize_canonical_and_inner() {
    assert_eq!(serde_json::to_string(&Fancy::S2Cell).unwrap(), "\"s2cell\"");
    assert_eq!(
        serde_json::to_string(&Fancy::Custom("foo".to_string())).unwrap(),
        "\"foo\""
    );
}

#[test]
fn fancy_deserialize_never_errors_with_default() {
    // Canonical, alias, and unknown all deserialize successfully.
    let a: Fancy = serde_json::from_str("\"S2\"").unwrap();
    assert_eq!(a, Fancy::S2Cell);
    let b: Fancy = serde_json::from_str("\"whatever\"").unwrap();
    assert_eq!(b, Fancy::Custom("whatever".to_string()));
}
