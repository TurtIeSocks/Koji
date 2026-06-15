//! The login wire struct for the session-auth surface.
//!
//! `Auth` (the login password body) was moved verbatim out of the dissolving
//! `model` crate. It is a plain serde DTO with no behavior — re-homed here so
//! `model` has no remaining koji-service importers. Consumed by the v2 auth
//! handler (`POST /api/v2/auth/login`).

use serde::{Deserialize, Serialize};

/// Login request body for `POST /api/v2/auth/login`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Auth {
    pub password: String,
}
