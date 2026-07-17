//! HTTP wire layer for the v2 request surface — now a re-export of the shared
//! [`koji_calc_api`] crate, extracted so the server and a wasm demo-mode
//! consumer share identical calc request semantics (input enums, arg-groups,
//! resolve helpers, per-op requests, and the tagged `CalcRequest`).
//!
//! The module is kept so existing `crate::requests::*` paths keep compiling;
//! new code may import from `koji_calc_api` directly.

pub use koji_calc_api::*;
