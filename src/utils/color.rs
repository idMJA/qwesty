#[must_use]
pub fn parse_color(hex: &str, fallback: u32) -> u32 {
    let hex = hex.trim_start_matches('#');
    u32::from_str_radix(hex, 16).unwrap_or(fallback)
}
