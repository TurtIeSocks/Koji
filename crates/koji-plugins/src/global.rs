//! Process-global, replaceable plugin registry.
//!
//! Defaults to a lazy disk scan ([`PluginRegistry::from_env`]) for back-compat;
//! `koji-service` replaces it via [`install`] once it has merged the DB overlay.

use std::sync::{Arc, OnceLock, RwLock};

use crate::registry::PluginRegistry;

fn cell() -> &'static RwLock<Arc<PluginRegistry>> {
    static REGISTRY: OnceLock<RwLock<Arc<PluginRegistry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Arc::new(PluginRegistry::from_env())))
}

/// Replace the process-global registry (called at startup + after each write).
pub fn install(registry: PluginRegistry) {
    *cell().write().expect("registry lock poisoned") = Arc::new(registry);
}

/// A snapshot of the current registry (cheap `Arc` clone).
pub fn current() -> Arc<PluginRegistry> {
    Arc::clone(&cell().read().expect("registry lock poisoned"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PluginKind, registry::Overlay};

    #[test]
    fn install_then_current_reflects_replacement() {
        let mut reg = PluginRegistry::default();
        reg.insert_manifest_for_test(PluginKind::Routing, "demo");
        reg.set_overlay(
            PluginKind::Routing,
            "demo",
            Overlay {
                enabled: false,
                args_default: None,
            },
        );
        install(reg);
        assert!(!current().is_enabled(PluginKind::Routing, "demo"));
    }
}
