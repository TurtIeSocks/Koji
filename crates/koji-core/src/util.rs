use crate::Precision;

pub trait TrimPrecision {
    fn trim_precision(self, precision: u32) -> Self;
}

impl TrimPrecision for Precision {
    fn trim_precision(self, precision: u32) -> Precision {
        if !self.is_finite() {
            return self;
        }
        let precision_factor = 10u32.pow(precision) as Precision;
        (self * precision_factor).round() / precision_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_precision_rounds_correctly() {
        assert_eq!(1.23456789f64.trim_precision(6), 1.234568);
        assert_eq!(1.23456789f64.trim_precision(4), 1.2346);
        assert_eq!(1.23456789f64.trim_precision(2), 1.23);
        assert_eq!(1.23456789f64.trim_precision(0), 1.0);
    }

    #[test]
    fn trim_precision_negative() {
        assert_eq!((-1.23456789f64).trim_precision(4), -1.2346);
    }

    #[test]
    fn trim_precision_exactly_representable() {
        assert_eq!(1.5f64.trim_precision(1), 1.5);
        assert_eq!(0.0f64.trim_precision(6), 0.0);
    }

    #[test]
    fn trim_precision_nan_passthrough() {
        let v = Precision::NAN.trim_precision(6);
        assert!(v.is_nan());
    }

    #[test]
    fn trim_precision_inf_passthrough() {
        let v = Precision::INFINITY.trim_precision(6);
        assert!(v.is_infinite() && v.is_sign_positive());
    }

    #[test]
    fn trim_precision_neg_inf_passthrough() {
        let v = Precision::NEG_INFINITY.trim_precision(6);
        assert!(v.is_infinite() && v.is_sign_negative());
    }
}
