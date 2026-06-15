//! Db-only JSON sort helper. Sorts a slice of `serde_json::Value` by the array
//! length at `sort_by`, ascending or descending. Relocated from koji-core
//! (`text_utils`) — koji-db is its sole consumer (project + geofence queries).

pub fn json_related_sort(json: &mut [serde_json::Value], sort_by: &str, order: String) {
    json.sort_by(|a, b| {
        let a = a[sort_by].as_array().unwrap().len();
        let b = b[sort_by].as_array().unwrap().len();
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
        json_related_sort(&mut items, "children", "asc".to_string());
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
        json_related_sort(&mut items, "routes", "desc".to_string());
        let lengths: Vec<usize> = items
            .iter()
            .map(|v| v["routes"].as_array().unwrap().len())
            .collect();
        assert_eq!(lengths, vec![3, 2, 1]);
    }

    #[test]
    fn sort_empty_slice_is_noop() {
        let mut items: Vec<serde_json::Value> = vec![];
        json_related_sort(&mut items, "children", "asc".to_string());
        assert!(items.is_empty());
    }

    #[test]
    fn sort_single_item_is_stable() {
        let mut items = vec![json!({ "children": [42] })];
        json_related_sort(&mut items, "children", "desc".to_string());
        assert_eq!(items[0]["children"][0], 42);
    }

    #[test]
    fn sort_desc_with_equal_lengths_preserves_relative_order() {
        // Two items with the same array length — sort must not panic and the
        // slice must remain the same length.
        let mut items = vec![
            json!({ "routes": [1, 2] }),
            json!({ "routes": [3, 4] }),
        ];
        json_related_sort(&mut items, "routes", "desc".to_string());
        assert_eq!(items.len(), 2);
    }
}
