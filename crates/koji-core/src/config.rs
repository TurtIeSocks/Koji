//! Composable algorithm/request configuration structs. `model`'s legacy `Args`
//! (and, later, the v2 request DTOs) map into these; the algorithms consume
//! them from P2 onward.

use crate::{ReturnTypeArg, SpawnpointTth};

#[derive(Debug, Clone)]
pub struct DataFilter {
    pub last_seen: u32,
    pub tth: SpawnpointTth,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub return_type: ReturnTypeArg,
    pub save_to_db: bool,
    pub save_to_scanner: bool,
    pub save_to_scanner_only: bool,
    pub simplify: bool,
}

/// Developer / experimental toggles + benchmark mode.
#[derive(Debug, Clone, Default)]
pub struct DevConfig {
    pub bypass_adaptive_partition: bool,
    pub benchmark_mode: bool,
}
