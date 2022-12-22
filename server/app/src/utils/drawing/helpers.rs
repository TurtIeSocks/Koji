use geo::Coord;

pub trait Helpers {
    fn vincenty_inverse(&self, end: &Coord) -> f64;
}

impl Helpers for Coord {
    fn vincenty_inverse(&self, end: &Coord) -> f64 {
        let a: f64 = 6378137.0; // Semi-major axis
        let b: f64 = 6356752.314245; // Semi-minor axis
        let f: f64 = 1.0 / 298.257223563; // Inverse-flattening

        // Start and end points in Radians
        let p1 = (self.y.to_radians(), self.x.to_radians());
        let p2 = (end.y.to_radians(), end.x.to_radians());

        // Difference in longitudes
        let l = p2.1 - p1.1;

        // u = 'reduced latitude'
        let (tan_u1, tan_u2) = ((1.0 - f) * p1.0.tan(), (1.0 - f) * p2.0.tan());
        let (cos_u1, cos_u2) = (
            1.0 / (1.0 + tan_u1 * tan_u1).sqrt(),
            1.0 / (1.0 + tan_u2 * tan_u2).sqrt(),
        );
        let (sin_u1, sin_u2) = (tan_u1 * cos_u1, tan_u2 * cos_u2);

        // First approximation
        let mut lambda = l;
        let mut iter_limit = 100;
        let mut cos_sq_alpha = 0.0;
        let (mut sin_sigma, mut cos_sigma, mut cos2_sigma_m, mut sigma) = (0.0, 0.0, 0.0, 0.0);
        let (mut _sin_lambda, mut _cos_lambda) = (0.0, 0.0);
        loop {
            _sin_lambda = lambda.sin();
            _cos_lambda = lambda.cos();
            let sin_sq_sigma = (cos_u2 * _sin_lambda) * (cos_u2 * _sin_lambda)
                + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * _cos_lambda)
                    * (cos_u1 * sin_u2 - sin_u1 * cos_u2 * _cos_lambda);

            // Points coincide
            if sin_sq_sigma == 0.0 {
                break;
            }

            sin_sigma = sin_sq_sigma.sqrt();
            cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * _cos_lambda;
            sigma = sin_sigma.atan2(cos_sigma);
            let sin_alpha = cos_u1 * cos_u2 * _sin_lambda / sin_sigma;
            cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;
            cos2_sigma_m = if cos_sq_alpha != 0.0 {
                cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha
            } else {
                0.0
            };
            let c = f / 16.0 * cos_sq_alpha * (4.0 + f * (4.0 - 3.0 * cos_sq_alpha));
            let lambda_prime = lambda;
            lambda = l
                + (1.0 - c)
                    * f
                    * sin_alpha
                    * (sigma
                        + c * sin_sigma
                            * (cos2_sigma_m
                                + c * cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)));

            iter_limit -= 1;
            if (lambda - lambda_prime).abs() > 1e-12 && iter_limit > 0 {
                continue;
            }

            break;
        }

        if iter_limit <= 0 {
            return f64::INFINITY;
        }

        let u_sq = cos_sq_alpha * (a * a - b * b) / (b * b);
        let cap_a =
            1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
        let cap_b = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));

        let delta_sigma = cap_b
            * sin_sigma
            * (cos2_sigma_m
                + cap_b / 4.0
                    * (cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)
                        - cap_b / 6.0
                            * cos2_sigma_m
                            * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                            * (-3.0 + 4.0 * cos2_sigma_m * cos2_sigma_m)));
        let s = b * cap_a * (sigma - delta_sigma);
        (s * 1000.0).round() / 1000.0
    }
}
