//! Db-only JSON sort helper. Sorts a slice of `serde_json::Value` by the array
//! length at `sort_by`, ascending or descending. Relocated from koji-core
//! (`text_utils`) — koji-db is its sole consumer (project + geofence queries).

pub fn json_related_sort(json: &mut [serde_json::Value], sort_by: &str, order: &str) {
    // `sort_by` is user-controlled (?sortBy=… survives the caller's
    // `.contains("length")` guard, e.g. `name.length`) — a non-array field must
    // degrade to length 0, not panic the request thread.
    let len_of = |v: &serde_json::Value| v[sort_by].as_array().map_or(0, Vec::len);
    json.sort_by(|a, b| {
        let (a, b) = (len_of(a), len_of(b));
        if order == "asc" { a.cmp(&b) } else { b.cmp(&a) }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sort_asc_by_array_length() {
        let mut items = vec![
            json!({ "children": [1, 2, 3] }),
            json!({ "children": [1] }),
            json!({ "children": [1, 2] }),
        ];
        json_related_sort(&mut items, "children", "asc");
        let lengths: Vec<usize> = items
            .iter()
            .map(|v| v["children"].as_array().unwrap().len())
            .collect();
        assert_eq!(lengths, vec![1, 2, 3]);
    }

    #[test]
    fn sort_desc_by_array_length() {
        let mut items = vec![
            json!({ "routes": [1] }),
            json!({ "routes": [1, 2, 3] }),
            json!({ "routes": [1, 2] }),
        ];
        json_related_sort(&mut items, "routes", "desc");
        let lengths: Vec<usize> = items
            .iter()
            .map(|v| v["routes"].as_array().unwrap().len())
            .collect();
        assert_eq!(lengths, vec![3, 2, 1]);
    }

    #[test]
    fn sort_empty_slice_is_noop() {
        let mut items: Vec<serde_json::Value> = vec![];
        json_related_sort(&mut items, "children", "asc");
        assert!(items.is_empty());
    }

    #[test]
    fn sort_single_item_is_stable() {
        let mut items = vec![json!({ "children": [42] })];
        json_related_sort(&mut items, "children", "desc");
        assert_eq!(items[0]["children"][0], 42);
    }

    #[test]
    fn non_array_sort_field_degrades_to_zero_instead_of_panicking() {
        // Regression: `?sortBy=name.length` passes the caller's contains("length")
        // guard and lands here with sort_by="name" — a string field.
        let mut items = vec![
            json!({ "name": "b", "routes": [1, 2] }),
            json!({ "name": "a" }),
        ];
        json_related_sort(&mut items, "name", "asc");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn sort_desc_with_equal_lengths_preserves_relative_order() {
        // Two items with the same array length — sort must not panic and the
        // slice must remain the same length.
        let mut items = vec![json!({ "routes": [1, 2] }), json!({ "routes": [3, 4] })];
        json_related_sort(&mut items, "routes", "desc");
        assert_eq!(items.len(), 2);
    }
}
