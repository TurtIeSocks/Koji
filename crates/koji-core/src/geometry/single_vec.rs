use crate::TrimPrecision;

use super::*;

pub type SingleVec = Vec<point_array::PointArray>;

impl EnsurePoints for SingleVec {
    fn ensure_first_last(self) -> Self {
        if self.is_empty() {
            return self;
        }
        let mut points = self;

        if points[0] != points[points.len() - 1] {
            points.push(points[0]);
        }
        points
    }
}

impl GetBbox for SingleVec {
    /// \[min_lon, min_lat, max_lon, max_lat\]
    fn get_bbox(&self) -> Option<Vec<Precision>> {
        let mut bbox = if self.is_empty() {
            vec![0., 0., 0., 0.]
        } else {
            vec![
                Precision::INFINITY,
                Precision::INFINITY,
                Precision::NEG_INFINITY,
                Precision::NEG_INFINITY,
            ]
        };

        for point in self {
            if point[1] < bbox[0] {
                bbox[0] = point[1]
            }
            if point[1] > bbox[2] {
                bbox[2] = point[1]
            }
            if point[0] < bbox[1] {
                bbox[1] = point[0]
            }
            if point[0] > bbox[3] {
                bbox[3] = point[0]
            }
        }
        Some(bbox.into_iter().map(|e| e.trim_precision(6)).collect())
    }
}
