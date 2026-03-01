#[must_use]
pub fn select_quest_accent_color(primary: &str, secondary: &str, fallback: u32) -> u32 {
    if let Some(primary_rgb) = parse_color_rgb(primary) {
        if !is_too_light(primary_rgb) {
            return primary_rgb;
        }
    }

    if let Some(secondary_rgb) = parse_color_rgb(secondary) {
        if !is_too_light(secondary_rgb) {
            return secondary_rgb;
        }
    }

    fallback
}

fn parse_color_rgb(hex: &str) -> Option<u32> {
    let sanitized = hex.trim().trim_start_matches('#');

    match sanitized.len() {
        6 => u32::from_str_radix(sanitized, 16).ok(),
        8 => {
            let value = u32::from_str_radix(sanitized, 16).ok()?;
            Some(value & 0x00FF_FFFF)
        }
        3 => {
            let mut expanded = String::with_capacity(6);
            for ch in sanitized.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            u32::from_str_radix(&expanded, 16).ok()
        }
        _ => None,
    }
}

const fn is_too_light(rgb: u32) -> bool {
    let r = ((rgb >> 16) & 0xFF) as u16;
    let g = ((rgb >> 8) & 0xFF) as u16;
    let b = (rgb & 0xFF) as u16;
    let luma = (299 * r + 587 * g + 114 * b) / 1000;
    luma >= 245
}
