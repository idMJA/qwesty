#[must_use]
pub fn parse_timestamp(iso_timestamp: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(iso_timestamp).map_or(0, |dt| dt.timestamp())
}
