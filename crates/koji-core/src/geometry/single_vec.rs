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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_first_last_open_ring_closes() {
        let v: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]];
        let closed = v.ensure_first_last();
        assert_eq!(closed.len(), 4);
        assert_eq!(closed[0], closed[closed.len() - 1]);
    }

    #[test]
    fn ensure_first_last_already_closed_unchanged() {
        let v: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [1.0, 2.0]];
        let out = v.clone().ensure_first_last();
        assert_eq!(out, v, "already-closed ring must not add a duplicate");
    }

    #[test]
    fn ensure_first_last_empty_unchanged() {
        let v: SingleVec = vec![];
        let out = v.ensure_first_last();
        assert!(out.is_empty());
    }

    #[test]
    fn ensure_first_last_single_point_unchanged() {
        // Single point: first == last (same element), no extra push.
        let v: SingleVec = vec![[7.0, 8.0]];
        let out = v.ensure_first_last();
        assert_eq!(out, vec![[7.0, 8.0]]);
    }

    #[test]
    fn ensure_first_last_two_different_points_closes() {
        let v: SingleVec = vec![[0.0, 0.0], [1.0, 1.0]];
        let out = v.ensure_first_last();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], out[2]);
    }
}
