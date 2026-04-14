//! Theme system.

pub mod contrast;
pub mod tokens;

pub use tokens::Theme;

use ratatui::style::Color;

impl Theme {
    /// Dark theme with Section 508-compliant contrast ratios.
    pub fn dark() -> Self {
        Self {
            // Surfaces — deep grays
            surface: Color::Rgb(30, 30, 35),
            surface_raised: Color::Rgb(45, 45, 50),
            surface_sunken: Color::Rgb(20, 20, 23),

            // On-surface text — high contrast light tones
            on_surface: Color::Rgb(220, 220, 225),
            on_surface_dim: Color::Rgb(160, 160, 170),
            on_surface_strong: Color::Rgb(250, 250, 250),

            // Primary — cyan-teal (darkened so white fg clears 4.5:1)
            primary: Color::Rgb(0, 105, 140),
            on_primary: Color::Rgb(255, 255, 255),
            primary_dim: Color::Rgb(80, 160, 190),

            // Secondary — muted blue
            secondary: Color::Rgb(100, 120, 160),
            on_secondary: Color::Rgb(240, 240, 245),

            // Semantic
            error: Color::Rgb(255, 110, 110),
            on_error: Color::Rgb(30, 0, 0),
            success: Color::Rgb(80, 220, 100),
            warning: Color::Rgb(250, 200, 80),
            info: Color::Rgb(120, 180, 230),

            // Borders & decoration
            border: Color::Rgb(80, 80, 90),
            border_focused: Color::Rgb(0, 105, 140),
            divider: Color::Rgb(55, 55, 60),
            cursor: Color::Rgb(255, 255, 255),
        }
    }

    /// Light theme with Section 508-compliant contrast ratios.
    pub fn light() -> Self {
        Self {
            surface: Color::Rgb(250, 250, 252),
            surface_raised: Color::Rgb(255, 255, 255),
            surface_sunken: Color::Rgb(240, 240, 245),

            on_surface: Color::Rgb(30, 30, 35),
            on_surface_dim: Color::Rgb(90, 90, 100),
            on_surface_strong: Color::Rgb(0, 0, 0),

            primary: Color::Rgb(0, 95, 135),
            on_primary: Color::Rgb(255, 255, 255),
            primary_dim: Color::Rgb(60, 130, 170),

            secondary: Color::Rgb(80, 100, 140),
            on_secondary: Color::Rgb(255, 255, 255),

            error: Color::Rgb(180, 30, 30),
            on_error: Color::Rgb(255, 255, 255),
            success: Color::Rgb(40, 140, 60),
            warning: Color::Rgb(170, 110, 20),
            info: Color::Rgb(40, 100, 180),

            border: Color::Rgb(200, 200, 205),
            border_focused: Color::Rgb(0, 95, 135),
            divider: Color::Rgb(220, 220, 225),
            cursor: Color::Rgb(0, 0, 0),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast::contrast_ratio;

    fn assert_aa(label: &str, fg: Color, bg: Color) {
        let r = contrast_ratio(fg, bg).unwrap_or(0.0);
        assert!(
            r >= 4.5,
            "{} contrast {:.2} fails AA (need ≥4.5)",
            label,
            r
        );
    }

    #[test]
    fn dark_theme_passes_aa() {
        let t = Theme::dark();
        assert_aa("on_surface/surface", t.on_surface, t.surface);
        assert_aa("on_primary/primary", t.on_primary, t.primary);
        assert_aa("on_error/error", t.on_error, t.error);
        assert_aa("on_surface_strong/surface", t.on_surface_strong, t.surface);
    }

    #[test]
    fn light_theme_passes_aa() {
        let t = Theme::light();
        assert_aa("on_surface/surface", t.on_surface, t.surface);
        assert_aa("on_primary/primary", t.on_primary, t.primary);
        assert_aa("on_error/error", t.on_error, t.error);
    }
}
