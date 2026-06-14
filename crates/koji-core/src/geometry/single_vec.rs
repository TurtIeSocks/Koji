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
