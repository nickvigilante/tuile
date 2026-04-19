//! # tuile
//!
//! A reusable TUI component framework built on ratatui.
//!
//! See crate README for design goals.

pub mod action;
pub mod component;
pub mod containers;
pub mod event;
pub mod focus;
pub mod scroll_content;
pub mod theme;
pub mod validation;
pub mod widgets;

// Re-exports (uncommented as types are implemented):
pub use action::Action;
pub use component::{Component, RenderContext};
pub use event::Event;
pub use focus::FocusManager;
pub use scroll_content::ScrollContent;
pub use theme::Theme;
pub use validation::ValidationResult;
