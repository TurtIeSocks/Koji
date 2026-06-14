//! Bootstrap algorithm-parameter config. `calculation_mode`/`s2` reuse the
//! clustering-owned shared types via `crate::clustering::{CalculationMode, S2Config}`.

use koji_core::Precision;

use crate::clustering::{CalculationMode, S2Config};

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub calculation_mode: CalculationMode,
    pub radius: Precision,
    pub s2: S2Config,
    pub plugin_args: String,
}
