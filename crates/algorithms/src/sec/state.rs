use geo::Point;

#[derive(Debug)]
pub enum State {
    S0,
    S1,
    S2(Point),
    S3(Point),
    S4,
}
