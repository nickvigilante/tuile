//! WCAG contrast ratio computation for validating theme accessibility.

use ratatui::style::Color;

/// Compute the WCAG 2.1 relative luminance of an RGB color (0.0–1.0).
fn luminance(r: u8, g: u8, b: u8) -> f64 {
    fn linearize(c: u8) -> f64 {
        let v = c as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

/// Compute the WCAG contrast ratio between two colors. Returns a value in
/// [1.0, 21.0]. Section 508 / WCAG AA requires ≥4.5 for normal text,
/// ≥3.0 for large text.
pub fn contrast_ratio(fg: Color, bg: Color) -> Option<f64> {
    let (fr, fg_, fb) = rgb_components(fg)?;
    let (br, bg_, bb) = rgb_components(bg)?;
    let l1 = luminance(fr, fg_, fb);
    let l2 = luminance(br, bg_, bb);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    Some((lighter + 0.05) / (darker + 0.05))
}

fn rgb_components(c: Color) -> Option<(u8, u8, u8)> {
    match c {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        // Can't compute for palette/named colors; terminal decides.
        _ => None,
    }
}

/// Check whether a foreground/background pair meets Section 508 (≥4.5:1).
pub fn meets_aa(fg: Color, bg: Color) -> bool {
    contrast_ratio(fg, bg).is_none_or(|r| r >= 4.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_on_black_is_max() {
        let r = contrast_ratio(Color::Rgb(255, 255, 255), Color::Rgb(0, 0, 0)).unwrap();
        assert!((r - 21.0).abs() < 0.1);
    }

    #[test]
    fn same_color_is_min() {
        let r = contrast_ratio(Color::Rgb(128, 128, 128), Color::Rgb(128, 128, 128)).unwrap();
        assert!((r - 1.0).abs() < 0.01);
    }

    #[test]
    fn named_colors_return_none() {
        assert_eq!(contrast_ratio(Color::White, Color::Black), None);
    }
}
