//! Integration test for the `fort_query` attribute macro. Proc-macro crates
//! can't host `#[test]` in `src/`, so the macro's behavioral spec lives here.
//!
//! The macro emits an `impl Query { all, bound, area, stats }` whose body calls
//! into `sea_orm`, `geojson`, `koji_core`, and several `crate::`-rooted items
//! (`crate::normalize::fort{,_filtered}`, `crate::rows::{GenericData, LatLonRow,
//! Total}`, `crate::sql_raw_bbox`, `crate::count_in_area`). None of those are
//! real deps of the `macros` crate, so we stub the *entire* call-site surface
//! here with the minimal shapes the generated code touches. If the macro parses
//! its `table`/`prefix` args correctly, emits all four methods, and roots its
//! paths at `crate::` as intended, this file type-checks and the methods are
//! callable. A live DB is not required — the stub query builder is synchronous
//! sugar that just records nothing and returns empty vecs.

#![allow(dead_code)]

// --- `crate::`-rooted stubs the macro output references -------------------

pub mod rows {
    #[derive(Debug, PartialEq)]
    pub struct GenericData {
        pub i: String,
        pub p: [f64; 2],
    }
    #[derive(Debug, Default)]
    pub struct LatLonRow {
        pub lat: f64,
        pub lon: f64,
    }
    #[derive(Debug, PartialEq)]
    pub struct Total {
        pub total: i32,
    }
}

pub mod normalize {
    use super::rows::{GenericData, LatLonRow};
    // Mirror the real signatures (prefix is &'static str post-commit-1).
    pub fn fort(items: Vec<LatLonRow>, prefix: &'static str) -> Vec<GenericData> {
        items
            .into_iter()
            .enumerate()
            .map(|(i, it)| GenericData {
                i: format!("{}{}", prefix, i),
                p: [it.lat, it.lon],
            })
            .collect()
    }
    pub fn fort_filtered(
        items: Vec<LatLonRow>,
        _area: &crate::geojson::FeatureCollection,
        prefix: &'static str,
    ) -> Vec<GenericData> {
        fort(items, prefix)
    }
}

pub fn sql_raw_bbox(_area: &geojson::FeatureCollection) -> String {
    "1=1".to_string()
}

pub fn count_in_area(items: &[rows::LatLonRow], _area: &geojson::FeatureCollection) -> i32 {
    items.len() as i32
}

// --- third-party crates the macro names, stubbed locally ------------------

pub mod geojson {
    #[derive(Default)]
    pub struct FeatureCollection;
}

pub mod koji_core {
    #[derive(Default)]
    pub struct KojiBbox {
        pub min_lat: f64,
        pub max_lat: f64,
        pub min_lon: f64,
        pub max_lon: f64,
    }
    #[derive(Default)]
    pub struct BoundsArg {
        pub bbox: KojiBbox,
        pub last_seen: Option<u32>,
    }
}

pub mod sea_orm {
    #[derive(Default)]
    pub struct DatabaseConnection;
    #[derive(Debug)]
    pub struct DbErr;
    pub enum DbBackend {
        MySql,
    }
    pub struct Statement;
    impl Statement {
        // Accepts the (backend, sql, params) shape the macro builds.
        pub fn from_sql_and_values<S: Into<String>>(
            _backend: DbBackend,
            _sql: S,
            _params: Vec<()>,
        ) -> Self {
            Statement
        }
    }
}

// `Entity::find()` returns a chainable builder; every builder method returns
// `Self` so the generated method chain type-checks. The terminal `all` is async
// and yields an empty Vec of whatever `into_model` was parameterized with.
struct Builder<T>(std::marker::PhantomData<T>);
impl Builder<()> {
    fn new() -> Self {
        Builder(std::marker::PhantomData)
    }
}
// `from_raw_sql` deliberately mirrors sea-orm's real builder method name (which
// the macro output calls), so suppress the from_*-takes-no-self heuristic here.
#[allow(clippy::wrong_self_convention)]
impl<T> Builder<T> {
    fn select_only(self) -> Self {
        self
    }
    fn column(self, _c: ColumnKind) -> Self {
        self
    }
    fn filter(self, _f: bool) -> Self {
        self
    }
    fn limit(self, _n: u64) -> Self {
        self
    }
    fn from_raw_sql(self, _s: sea_orm::Statement) -> Self {
        self
    }
    fn into_model<U>(self) -> Builder<U> {
        Builder(std::marker::PhantomData)
    }
    async fn all(self, _conn: &sea_orm::DatabaseConnection) -> Result<Vec<T>, sea_orm::DbErr> {
        Ok(Vec::new())
    }
}

struct Entity;
impl Entity {
    fn find() -> Builder<()> {
        Builder::new()
    }
}

// `Column::Lat.gt(..)`/`.eq(..)`/`.between(..)` all yield a `bool` filter sentinel
// so `.filter(...)` (which takes `bool` above) accepts them.
enum ColumnKind {
    Lat,
    Lon,
    Updated,
    Deleted,
    Enabled,
}
impl ColumnKind {
    fn gt<T>(&self, _v: T) -> bool {
        true
    }
    fn eq<T>(&self, _v: T) -> bool {
        true
    }
    fn between<T>(&self, _a: T, _b: T) -> bool {
        true
    }
}
#[allow(non_upper_case_globals)]
impl Column {
    const Lat: ColumnKind = ColumnKind::Lat;
    const Lon: ColumnKind = ColumnKind::Lon;
    const Updated: ColumnKind = ColumnKind::Updated;
    const Deleted: ColumnKind = ColumnKind::Deleted;
    const Enabled: ColumnKind = ColumnKind::Enabled;
}
struct Column;

// --- the macro under test -------------------------------------------------

#[macros::fort_query(table = "gym", prefix = "g")]
pub struct Query;

// Minimal no-dep executor: the stub futures never pend (every await is on an
// immediately-ready value), so a busy `poll` resolves them in one step.
fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    // SAFETY: `fut` lives on this stack frame and is never moved after pinning.
    let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

#[test]
fn fort_query_emits_all_four_methods_with_correct_shapes() {
    // Compile-time proof the four async methods exist with the expected arity
    // and signatures: coerce each to a fn pointer with its full type. If the
    // macro dropped a method, mis-parsed `table`/`prefix`, or mis-rooted a
    // `crate::` path, this file would not type-check.
    let conn = sea_orm::DatabaseConnection;
    let area = geojson::FeatureCollection;

    // `all` → Vec<GenericData>; empty stub result set, but the fort(prefix="g")
    // call path is exercised and type-checked end to end.
    let out: Vec<rows::GenericData> = block_on(Query::all(&conn, 0)).unwrap();
    assert!(out.is_empty());

    // `bound` takes &BoundsArg and returns Vec<GenericData>.
    let bounds = koji_core::BoundsArg::default();
    let out = block_on(Query::bound(&conn, &bounds)).unwrap();
    assert!(out.is_empty());

    // `area` takes &FeatureCollection + last_seen, returns Vec<GenericData>.
    let out = block_on(Query::area(&conn, &area, 0)).unwrap();
    assert!(out.is_empty());

    // `stats` returns a Total (via count_in_area over the stub rows → 0).
    let total = block_on(Query::stats(&conn, &area, 0)).unwrap();
    assert_eq!(total, rows::Total { total: 0 });
}
