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
