//! The `ScrollContent` sub-trait. Widgets implement this in addition to
//! `Component` if they are safe to place inside a `ScrollView`. Widgets
//! that do not implement it cannot be used as the direct child of a
//! `ScrollView` — you will get a compile error like:
//!
//! ```text
//! the trait `ScrollContent` is not implemented for `TextField`
//! ```
//!
//! This is intentional. Editable widgets (with cursors) and open-overlay
//! widgets (Dropdown) cannot be rendered correctly via buffer-only
//! rendering in v0.2.
//!
//! ## Compile-fail guard
//!
//! The following snippet must not compile — `TextField` does not implement
//! `ScrollContent`, so it cannot be the direct child of a `ScrollView`:
//!
//! ```compile_fail
//! use tuile::containers::scroll_view::ScrollView;
//! use tuile::widgets::text_field::TextField;
//! let _ = ScrollView::new(Box::new(TextField::new("label", "")));
//! ```

use crate::component::{Component, RenderContext};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub trait ScrollContent: Component {
    /// Natural height needed to fully render at the given width. Called by
    /// `ScrollView` to size its scratch buffer. Containers implement this
    /// recursively (e.g. `VStack` sums its children's measures).
    fn measure(&self, width: u16) -> u16;

    /// Render into a buffer at the given area. No cursor support — buffer
    /// rendering does not have access to `Frame::set_cursor_position`.
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext);
}
