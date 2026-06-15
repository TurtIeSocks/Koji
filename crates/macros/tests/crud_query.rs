//! Integration tests for the `#[crud_query]` attribute macro.
//!
//! The macro emits `impl Query { get_one, get_one_json, delete, search }` whose
//! bodies reference call-site-resolved symbols: `Entity`, `Column`, `Model`,
//! `Json`, `DatabaseConnection`, `DbErr`, `DeleteResult`, `crate::error::ModelError`,
//! `serde_json::json!`. All are stubbed minimally here.
//!
//! Since all generated method bodies are `async`, we run them via a minimal
//! single-threaded executor (the same pattern as fort_query.rs).

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Stubs for sea-orm symbols the macro references unqualified at the call site.
// ---------------------------------------------------------------------------

pub type Json = serde_json::Value;

#[derive(Debug, serde::Serialize)]
pub struct Model {
    pub id: u32,
    pub name: String,
}

pub struct DatabaseConnection;

#[derive(Debug)]
pub struct DbErr(pub String);

pub struct DeleteResult {
    pub rows_affected: u64,
}

// ---------------------------------------------------------------------------
// Stub `Entity` + `Column` + query builder.
// ---------------------------------------------------------------------------

pub struct Entity;
impl Entity {
    pub fn find() -> Builder {
        Builder { data: None }
    }
    pub fn find_by_id(_id: u32) -> Builder {
        Builder {
            data: Some(Model {
                id: 1,
                name: "found".to_string(),
            }),
        }
    }
    pub fn delete_by_id(_id: u32) -> DeleteBuilder {
        DeleteBuilder { rows: 1 }
    }
}

pub struct Builder {
    data: Option<Model>,
}

impl Builder {
    pub fn filter(self, _cond: bool) -> Self {
        self
    }
    pub fn like(self, _s: &str) -> Self {
        self
    }
    pub async fn one(self, _db: &DatabaseConnection) -> Result<Option<Model>, DbErr> {
        Ok(self.data)
    }
    pub fn into_json(self) -> JsonBuilder {
        JsonBuilder
    }
}

pub struct JsonBuilder;
impl JsonBuilder {
    pub async fn all(self, _db: &DatabaseConnection) -> Result<Vec<Json>, DbErr> {
        Ok(vec![])
    }
}

pub struct DeleteBuilder {
    rows: u64,
}
impl DeleteBuilder {
    pub async fn exec(self, _db: &DatabaseConnection) -> Result<DeleteResult, DbErr> {
        Ok(DeleteResult {
            rows_affected: self.rows,
        })
    }
}

// Column::Name is a const-ish value with .eq() / .like() that return bool.
#[derive(Clone, Copy)]
pub struct ColumnName;
impl ColumnName {
    pub fn eq(self, _v: impl std::fmt::Display) -> bool {
        true
    }
    pub fn like(self, _v: &str) -> bool {
        true
    }
}
pub struct Column;
#[allow(non_upper_case_globals)]
impl Column {
    pub const Name: ColumnName = ColumnName;
}

// ---------------------------------------------------------------------------
// crate::error::ModelError — the macro calls this fully-qualified.
// ---------------------------------------------------------------------------

pub mod error {
    #[derive(Debug)]
    pub enum ModelError {
        Geofence(String),
        Db(String),
    }
    impl std::fmt::Display for ModelError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ModelError::Geofence(s) => write!(f, "ModelError: {s}"),
                ModelError::Db(s) => write!(f, "ModelError: {s}"),
            }
        }
    }
    impl From<super::DbErr> for ModelError {
        fn from(e: super::DbErr) -> Self {
            ModelError::Db(e.0)
        }
    }
}

// ---------------------------------------------------------------------------
// The macro under test.
// ---------------------------------------------------------------------------

#[macros::crud_query]
pub struct Query;

// ---------------------------------------------------------------------------
// Minimal executor (identical to fort_query.rs).
// ---------------------------------------------------------------------------

fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn crud_query_get_one_by_numeric_id_returns_ok() {
    // "1" parses as u32 → Entity::find_by_id path.
    let db = DatabaseConnection;
    let result = block_on(Query::get_one(&db, "1".to_string()));
    assert!(result.is_ok(), "expected Ok, got {:?}", result.err().map(|e| e.to_string()));
}

#[test]
fn crud_query_get_one_by_name_not_found_returns_err() {
    // non-numeric → Entity::find().filter(Column::Name.eq(id)) path.
    // Stub Builder::one() returns Ok(None) → macro emits the Err("Does not exist") arm.
    let db = DatabaseConnection;
    let result = block_on(Query::get_one(&db, "nonexistent".to_string()));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Does not exist"));
}

#[test]
fn crud_query_get_one_json_returns_json_value() {
    let db = DatabaseConnection;
    // Numeric id resolves to a Model whose serde_json::json! wrapper produces a Value.
    let result = block_on(Query::get_one_json(&db, "1".to_string()));
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_object() || !val.is_null());
}

#[test]
fn crud_query_delete_returns_delete_result() {
    let db = DatabaseConnection;
    let result = block_on(Query::delete(&db, 1));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().rows_affected, 1);
}

#[test]
fn crud_query_search_returns_vec() {
    let db = DatabaseConnection;
    let result = block_on(Query::search(&db, "gym".to_string()));
    assert!(result.is_ok());
    // stub returns empty vec — just verify the method exists and returns Vec<Json>
    let items: Vec<Json> = result.unwrap();
    assert!(items.is_empty());
}
