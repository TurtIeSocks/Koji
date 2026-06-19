//! Temporary: proves the new deps link. Deleted once Task 6 wires the real hub.
#[test]
fn actix_ws_and_futures_link() {
    // Referencing the crate roots forces a link error if the dep is missing.
    let _ = std::any::type_name::<actix_ws::Session>();
    fn _uses_futures<S: futures_util::Stream>(_s: S) {}
}
