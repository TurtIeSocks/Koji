//! Page-based pagination for v2 list endpoints. `Pagination` parses
//! `?page=&per_page=` (clamped); `Meta::build` computes the `meta` block the
//! envelope carries.
//!
//! The wire is **1-based** (`?page=1` is the first page) — chosen to match
//! koji-db's existing `Query::paginate` / `AdminReqParsed` page-shaped surface
//! and keep the whole v2 list API consistent. `offset()` bridges to any
//! offset-based consumer (`offset = (page-1) * per_page`).
//!
//! Phase 0 builds these primitives ahead of their consumers: the v2 list
//! handlers that extract `Pagination` and call `Meta::build` /
//! `success_paginated` are regenerated in P2 (typed CRUD). Until then every item
//! here is exercised only by the unit tests below, so the non-test build sees it
//! as dead — silenced crate-wide for this module rather than item-by-item.
#![allow(dead_code)]

use serde::Deserialize;

use crate::utils::api_response::Meta;

const DEFAULT_PER_PAGE: i64 = 50;
const MAX_PER_PAGE: i64 = 500;

/// `?page=&per_page=` query for list endpoints. Both optional; access the
/// effective (clamped) values via [`Pagination::page`] / [`Pagination::per_page`]
/// (and [`Pagination::offset`] for offset-based consumers).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
}

impl Pagination {
    /// Build a `Pagination` from explicit `Option<i64>` parts.  Used by handlers
    /// that cannot use `#[serde(flatten)]` (e.g. `JobListQuery` where
    /// `serde_urlencoded` doesn't support flattening).
    pub(crate) fn from_parts(page: Option<i64>, per_page: Option<i64>) -> Self {
        Pagination { page, per_page }
    }

    /// Effective page (1-based): defaults to 1, floored at 1.
    pub(crate) fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Effective per-page: defaults to 50, clamped to `[1, 500]`.
    pub(crate) fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(DEFAULT_PER_PAGE).clamp(1, MAX_PER_PAGE)
    }

    /// Zero-based row offset for the effective page: `(page - 1) * per_page`.
    pub(crate) fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

impl Meta {
    /// Build the pagination `meta` from the total row count and the effective
    /// (1-based) `page` / `per_page` (both already clamped via [`Pagination`]).
    /// `page` 1 is the first page; `has_prev` is `page > 1`.
    pub(crate) fn build(total: i64, page: i64, per_page: i64) -> Meta {
        let per_page = per_page.max(1);
        let page = page.max(1);
        let total_pages = if total == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };
        Meta {
            total,
            page,
            per_page,
            total_pages,
            has_next: page * per_page < total,
            has_prev: page > 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_page_defaults_and_clamps() {
        assert_eq!(Pagination::default().per_page(), 50);
        assert_eq!(Pagination { page: None, per_page: Some(9999) }.per_page(), 500);
        assert_eq!(Pagination { page: None, per_page: Some(0) }.per_page(), 1);
    }

    #[test]
    fn page_defaults_and_floors_at_one() {
        assert_eq!(Pagination::default().page(), 1);
        assert_eq!(Pagination { page: Some(0), per_page: None }.page(), 1);
        assert_eq!(Pagination { page: Some(-5), per_page: None }.page(), 1);
        assert_eq!(Pagination { page: Some(3), per_page: None }.page(), 3);
    }

    #[test]
    fn offset_is_zero_based_from_one_based_page() {
        assert_eq!(Pagination::default().offset(), 0);
        assert_eq!(Pagination { page: Some(1), per_page: Some(50) }.offset(), 0);
        assert_eq!(Pagination { page: Some(2), per_page: Some(50) }.offset(), 50);
        assert_eq!(Pagination { page: Some(3), per_page: Some(20) }.offset(), 40);
    }

    #[test]
    fn meta_first_page_small_total() {
        let m = Meta::build(2, 1, 50);
        assert_eq!(m.total, 2);
        assert_eq!(m.page, 1);
        assert_eq!(m.per_page, 50);
        assert_eq!(m.total_pages, 1);
        assert!(!m.has_next);
        assert!(!m.has_prev);
    }

    #[test]
    fn meta_middle_page() {
        let m = Meta::build(120, 2, 50);
        assert_eq!(m.page, 2);
        assert_eq!(m.total_pages, 3);
        assert!(m.has_next);
        assert!(m.has_prev);
    }

    #[test]
    fn meta_last_page_no_next() {
        let m = Meta::build(120, 3, 50);
        assert_eq!(m.page, 3);
        assert_eq!(m.total_pages, 3);
        assert!(!m.has_next);
        assert!(m.has_prev);
    }

    #[test]
    fn meta_empty() {
        let m = Meta::build(0, 1, 50);
        assert_eq!(m.total_pages, 0);
        assert!(!m.has_next);
        assert!(!m.has_prev);
    }
}
