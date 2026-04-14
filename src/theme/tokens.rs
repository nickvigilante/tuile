//! Semantic theme tokens. Mapping to CSS custom properties:
//! - `surface` ≈ `background`
//! - `on_surface` ≈ `color`
//! - `primary` ≈ `--accent`
//! Widgets should use tokens, not hard-coded colors.

use ratatui::style::{Color, Modifier, Style};

/// Semantic color tokens. Widgets consume these; apps override them to retheme.
#[derive(Debug, Clone)]
pub struct Theme {
    // Surface colors — background layers
    pub surface: Color,
    pub surface_raised: Color,  // modals, dropdowns, tooltips
    pub surface_sunken: Color,  // input field backgrounds

    // Content colors — text on surfaces
    pub on_surface: Color,
    pub on_surface_dim: Color,  // secondary text, hints
    pub on_surface_strong: Color,  // headings, emphasized text

    // Accent — focus, active, selection
    pub primary: Color,
    pub on_primary: Color,
    pub primary_dim: Color,  // muted accent (unfocused selection)

    // Secondary — less emphasized interactive elements
    pub secondary: Color,
    pub on_secondary: Color,

    // Semantic roles
    pub error: Color,
    pub on_error: Color,
    pub success: Color,
    pub warning: Color,  // dirty markers, pending changes
    pub info: Color,

    // Borders & decoration
    pub border: Color,
    pub border_focused: Color,
    pub divider: Color,
    pub cursor: Color,
}

impl Theme {
    /// Get a default "focused input" style.
    pub fn focused_style(&self) -> Style {
        Style::default()
            .fg(self.on_primary)
            .bg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Get a default "unfocused input" style.
    pub fn unfocused_style(&self) -> Style {
        Style::default().fg(self.on_surface)
    }

    pub fn label_style(&self, focused: bool) -> Style {
        if focused {
            Style::default()
                .fg(self.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.on_surface_dim)
        }
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }
}
