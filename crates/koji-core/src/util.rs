pub trait TrimPrecision {
    fn trim_precision(self, precision: u32) -> Self;
}

impl TrimPrecision for f64 {
    fn trim_precision(self, precision: u32) -> f64 {
        if !self.is_finite() {
            return self;
        }
        let precision_factor = 10u32.pow(precision) as f64;
        (self * precision_factor).round() / precision_factor
    }
}
