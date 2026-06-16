use macros::StrEnum;

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
pub enum CalculationMode {
    #[str("radius")]
    Radius,
    #[str("s2")]
    S2,
    #[str(default)]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names_recognized() {
        assert_eq!(
            CalculationMode::from_str_opt("radius"),
            Some(CalculationMode::Radius)
        );
        assert_eq!(
            CalculationMode::from_str_opt("s2"),
            Some(CalculationMode::S2)
        );
    }

    #[test]
    fn unknown_string_becomes_custom() {
        let v = CalculationMode::from_str_opt("my_calc").unwrap();
        assert!(matches!(v, CalculationMode::Custom(ref s) if s == "my_calc"));
    }

    #[test]
    fn display_canonical() {
        assert_eq!(format!("{}", CalculationMode::Radius), "radius");
        assert_eq!(format!("{}", CalculationMode::S2), "s2");
    }

    #[test]
    fn serde_round_trip() {
        for mode in [
            CalculationMode::Radius,
            CalculationMode::S2,
            CalculationMode::Custom("x".into()),
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let back: CalculationMode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, mode);
        }
    }
}
