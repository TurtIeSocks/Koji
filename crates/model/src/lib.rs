//! model — transitional crate holding the calc-request `Args` and its
//! companions. Everything else (db, scanner, errors, utils, GeoFormats, query
//! args) has moved to koji-core / koji-db / koji-scanner. Dissolves in P1d when
//! `Args` breaks into config structs.

pub mod api;
