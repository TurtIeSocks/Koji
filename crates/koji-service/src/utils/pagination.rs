//! Offset pagination for v2 list endpoints. `Pagination` parses `?limit=&offset=`
//! (clamped); `Meta::build` computes the `meta` block the envelope carries.
//!
//! Phase 0 builds these primitives ahead of their consumers: the v2 list
//! handlers that extract `Pagination` and call `Meta::build` /
//! `success_paginated` are regenerated in P2 (typed CRUD). Until then every item
//! here is exercised only by the unit tests below, so the non-test build sees it
//! as dead — silenced crate-wide for this module rather than item-by-item.
#![allow(dead_code)]

use serde::Deserialize;

use crate::utils::api_response::Meta;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 500;

/// `?limit=&offset=` query for list endpoints. Both optional; access the
/// effective (clamped) values via [`Pagination::limit`] / [`Pagination::offset`].
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Pagination {
    limit: Option<i64>,
    offset: Option<i64>,
}

impl Pagination {
    /// Effective limit: defaults to 50, clamped to `[1, 500]`.
    pub(crate) fn limit(&self) -> i64 {
        self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
    }

    /// Effective offset: defaults to 0, never negative.
    pub(crate) fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

impl Meta {
    /// Build the pagination `meta` from the total row count and the effective
    /// `limit`/`offset` (both already clamped via [`Pagination`]). `page` is a
    /// 0-based page index.
    pub(crate) fn build(total: i64, limit: i64, offset: i64) -> Meta {
        let per_page = limit.max(1);
        let page = offset / per_page;
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
            has_next: offset + per_page < total,
            has_prev: offset > 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_defaults_and_clamps() {
        assert_eq!(Pagination::default().limit(), 50);
        assert_eq!(Pagination { limit: Some(9999), offset: None }.limit(), 500);
        assert_eq!(Pagination { limit: Some(0), offset: None }.limit(), 1);
    }

    #[test]
    fn offset_defaults_and_floors_at_zero() {
        assert_eq!(Pagination::default().offset(), 0);
        assert_eq!(Pagination { limit: None, offset: Some(-5) }.offset(), 0);
        assert_eq!(Pagination { limit: None, offset: Some(40) }.offset(), 40);
    }

    #[test]
    fn meta_first_page_small_total() {
        let m = Meta::build(2, 50, 0);
        assert_eq!(m.total, 2);
        assert_eq!(m.page, 0);
        assert_eq!(m.per_page, 50);
        assert_eq!(m.total_pages, 1);
        assert!(!m.has_next);
        assert!(!m.has_prev);
    }

    #[test]
    fn meta_middle_page() {
        let m = Meta::build(120, 50, 50);
        assert_eq!(m.page, 1);
        assert_eq!(m.total_pages, 3);
        assert!(m.has_next);
        assert!(m.has_prev);
    }

    #[test]
    fn meta_empty() {
        let m = Meta::build(0, 50, 0);
        assert_eq!(m.total_pages, 0);
        assert!(!m.has_next);
        assert!(!m.has_prev);
    }
}
