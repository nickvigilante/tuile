# Virtual-buffer ScrollView Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `ScrollView` actually scroll, by introducing a `ScrollContent` sub-trait and rendering children into an off-screen scratch `Buffer` that is then clipped to the viewport.

**Architecture:** Add a new trait `ScrollContent: Component` with `measure(width)` and `render_buf(buf, area, ctx)`. Add one optional method `Component::as_scroll_content(&self) -> Option<&dyn ScrollContent>` (default `None`) so containers can fan out to children generically. Refactor `ScrollView` to own a `RefCell<Buffer>` scratch, call `child.measure(area.width)` to size it, call `child.render_buf` into it, then copy the visible row-slice into the real frame buffer.

**Tech Stack:** Rust 2024, ratatui 0.29 (`Buffer`, `Widget::render`, `Paragraph::line_count`), crossterm 0.29. Tests use `ratatui::backend::TestBackend` for frame-level rendering and `Buffer::empty(Rect)` for scratch-level tests. No new dev-dependencies.

**Spec:** `docs/superpowers/specs/2026-04-18-virtual-buffer-scrollview-design.md`

---

## Task 1: Introduce `ScrollContent` trait and `Component::as_scroll_content`

**Files:**
- Create: `src/scroll_content.rs`
- Modify: `src/component.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src/component.rs` (inside the existing file — there is no test module yet; add one):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use crate::action::Action;
    use crate::event::Event;
    use ratatui::buffer::Buffer;

    struct Plain;
    impl Component for Plain {
        fn handle_event(&mut self, _: &Event, _: &mut Context) -> Action { Action::Ignored }
        fn render(&self, _: &mut ratatui::Frame, _: Rect, _: &RenderContext) {}
    }

    struct Scrollable;
    impl Component for Scrollable {
        fn handle_event(&mut self, _: &Event, _: &mut Context) -> Action { Action::Ignored }
        fn render(&self, _: &mut ratatui::Frame, _: Rect, _: &RenderContext) {}
        fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
    }
    impl ScrollContent for Scrollable {
        fn measure(&self, _: u16) -> u16 { 7 }
        fn render_buf(&self, _: &mut Buffer, _: Rect, _: &RenderContext) {}
    }

    #[test]
    fn plain_component_has_no_scroll_content() {
        let p = Plain;
        assert!(p.as_scroll_content().is_none());
    }

    #[test]
    fn scrollable_component_returns_self_as_scroll_content() {
        let s = Scrollable;
        let sc = s.as_scroll_content().expect("should be Some");
        assert_eq!(sc.measure(0), 7);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib component::tests`
Expected: FAIL — `ScrollContent` is not a type, `as_scroll_content` is not a method on `Component`.

- [ ] **Step 3: Create the ScrollContent trait**

Create `src/scroll_content.rs`:

```rust
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
```

- [ ] **Step 4: Add `as_scroll_content` to `Component`**

In `src/component.rs`, replace the `Component` trait block with the version below (keeps all existing methods, adds one). Keep the imports at the top of the file unchanged. Add one new `use` line:

```rust
use crate::scroll_content::ScrollContent;
```

Then the trait:

```rust
pub trait Component {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action;

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext);

    fn is_focusable(&self) -> bool { true }

    fn traps_focus(&self) -> bool { false }

    fn children_mut<'a>(&'a mut self) -> Vec<&'a mut (dyn Component + 'a)> {
        Vec::new()
    }

    fn children(&self) -> Vec<&dyn Component> {
        Vec::new()
    }

    fn name(&self) -> &'static str {
        "Component"
    }

    /// If this component also implements `ScrollContent`, return `Some(self)`.
    /// Default: `None`. Widgets that implement `ScrollContent` should
    /// override this to `Some(self)` so container types can delegate to
    /// them via `Box<dyn Component>` without a downcast.
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> {
        None
    }
}
```

- [ ] **Step 5: Wire module into `lib.rs`**

In `src/lib.rs`, add after `pub mod event;`:

```rust
pub mod scroll_content;
```

And add to the re-exports block:

```rust
pub use scroll_content::ScrollContent;
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test --lib`
Expected: all tests pass (previous 39 + 2 new = 41).

- [ ] **Step 7: Commit**

```bash
git add src/scroll_content.rs src/component.rs src/lib.rs
git commit -m "feat(scroll): add ScrollContent trait + Component::as_scroll_content"
```

---

## Task 2: `ScrollContent` impl for `Toggle`

**Files:**
- Modify: `src/widgets/toggle.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `src/widgets/toggle.rs`:

```rust
#[test]
fn scroll_content_measure_is_one() {
    use crate::scroll_content::ScrollContent;
    let t = Toggle::new("x", false);
    assert_eq!(t.measure(20), 1);
}

#[test]
fn scroll_content_render_buf_writes_on_label() {
    use crate::scroll_content::ScrollContent;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    let t = Toggle::new("x", true);
    let theme = crate::theme::Theme::dark();
    let rctx = RenderContext::new(&theme);
    let area = Rect::new(0, 0, 30, 1);
    let mut buf = Buffer::empty(area);
    t.render_buf(&mut buf, area, &rctx);
    // Label "x: " + marker "◉ ON" somewhere on row 0
    let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
    assert!(row.starts_with("x:"), "row was {:?}", row);
    assert!(row.contains("◉"), "row was {:?}", row);
    assert!(row.contains("ON"), "row was {:?}", row);
}

#[test]
fn as_scroll_content_returns_self() {
    let t = Toggle::new("x", false);
    assert!(t.as_scroll_content().is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::toggle::tests`
Expected: FAIL — `measure`, `render_buf`, and `ScrollContent` impl don't exist yet.

- [ ] **Step 3: Extract a paragraph builder and add `ScrollContent` impl**

In `src/widgets/toggle.rs`, replace the `Component::render` body with a small helper-based version and add the trait impls.

Add these imports near the top:

```rust
use crate::scroll_content::ScrollContent;
use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;
```

Replace the `impl Component for Toggle` block and add `impl ScrollContent for Toggle` after it:

```rust
impl Toggle {
    fn build_paragraph<'a>(&'a self, theme: &Theme) -> Paragraph<'a> {
        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };
        let (text, style) = if self.value {
            ("  ◉ ON ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
        } else {
            (" ○ OFF ", Style::default().fg(theme.on_surface_dim))
        };
        let line = Line::from(vec![
            Span::styled(format!("{}: ", self.label), theme.label_style(false)),
            Span::styled(text, style),
            dirty,
        ]);
        Paragraph::new(line)
    }
}

impl Component for Toggle {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        let Event::Key(k) = event else { return Action::Ignored; };
        match k.code {
            KeyCode::Enter | KeyCode::Char(' ') => { self.value = !self.value; Action::Changed }
            KeyCode::Esc => { self.revert(); Action::Cancel }
            _ => Action::Absorbed,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        frame.render_widget(self.build_paragraph(ctx.theme), area);
    }

    fn name(&self) -> &'static str { "Toggle" }

    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

impl ScrollContent for Toggle {
    fn measure(&self, _width: u16) -> u16 { 1 }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        self.build_paragraph(ctx.theme).render(area, buf);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::toggle::tests`
Expected: all toggle tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/toggle.rs
git commit -m "feat(scroll): implement ScrollContent for Toggle"
```

---

## Task 3: `ScrollContent` impl for `Radio`

**Files:**
- Modify: `src/widgets/radio.rs`

- [ ] **Step 1: Write the failing test**

Add a new `#[cfg(test)] mod tests` block at the bottom of `src/widgets/radio.rs` (there isn't one currently):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use ratatui::buffer::Buffer;

    #[test]
    fn scroll_content_measure_is_one() {
        let r = Radio::new("x", vec!["a".into(), "b".into()], 0);
        assert_eq!(r.measure(20), 1);
    }

    #[test]
    fn scroll_content_render_buf_writes_label_and_options() {
        let r = Radio::new("x", vec!["a".into(), "b".into()], 0);
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        r.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(row.contains("a"), "row was {:?}", row);
        assert!(row.contains("b"), "row was {:?}", row);
    }

    #[test]
    fn as_scroll_content_returns_self() {
        let r = Radio::new("x", vec!["a".into()], 0);
        assert!(r.as_scroll_content().is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::radio::tests`
Expected: FAIL — methods and impl don't exist yet.

- [ ] **Step 3: Add builder + `ScrollContent` impl**

In `src/widgets/radio.rs`, add near the top imports:

```rust
use crate::scroll_content::ScrollContent;
use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;
```

Add a builder method `build_paragraph` to `Radio` and replace the `render` body to use it:

```rust
impl Radio {
    fn build_paragraph<'a>(&'a self, theme: &Theme) -> Paragraph<'a> {
        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };

        let mut spans = vec![Span::styled(format!("{}: ", self.label), theme.label_style(false))];
        for (i, opt) in self.options.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { "◉" } else { "○" };
            let style = if is_selected {
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.on_surface_dim)
            };
            spans.push(Span::styled(format!(" {} {} ", marker, opt), style));
            if i + 1 < self.options.len() {
                spans.push(Span::styled("│", Style::default().fg(theme.divider)));
            }
        }
        spans.push(dirty);
        Paragraph::new(Line::from(spans))
    }
}
```

Replace the `fn render` body and add `as_scroll_content`:

```rust
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        frame.render_widget(self.build_paragraph(ctx.theme), area);
    }

    fn name(&self) -> &'static str { "Radio" }

    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
```

Add outside the `impl Component for Radio` block:

```rust
impl ScrollContent for Radio {
    fn measure(&self, _width: u16) -> u16 { 1 }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        self.build_paragraph(ctx.theme).render(area, buf);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::radio::tests`
Expected: all radio tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/radio.rs
git commit -m "feat(scroll): implement ScrollContent for Radio"
```

---

## Task 4: `ScrollContent` impl for `StatusBar`

**Files:**
- Modify: `src/widgets/status_bar.rs`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` block at the bottom of `src/widgets/status_bar.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use ratatui::buffer::Buffer;

    #[test]
    fn measure_is_one() {
        let b = StatusBar::new();
        assert_eq!(b.measure(20), 1);
    }

    #[test]
    fn render_buf_writes_text_when_visible() {
        let mut b = StatusBar::new();
        b.set("hello");
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        b.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(row.starts_with("hello"), "row was {:?}", row);
    }

    #[test]
    fn render_buf_writes_nothing_when_hidden() {
        let b = StatusBar::new(); // ticks == 0
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        b.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(row.trim(), "");
    }

    #[test]
    fn as_scroll_content_returns_self() {
        let b = StatusBar::new();
        assert!(b.as_scroll_content().is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::status_bar::tests`
Expected: FAIL.

- [ ] **Step 3: Add `ScrollContent` impl**

In `src/widgets/status_bar.rs`, add near the top imports:

```rust
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;
```

Inside the existing `impl Component for StatusBar` block, add the `as_scroll_content` override:

```rust
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
```

Add after the `impl Component for StatusBar` block:

```rust
impl ScrollContent for StatusBar {
    fn measure(&self, _width: u16) -> u16 { 1 }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        if !self.visible() { return; }
        Paragraph::new(Line::raw(self.text.clone()))
            .style(Style::default().fg(ctx.theme.info))
            .render(area, buf);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::status_bar::tests`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/widgets/status_bar.rs
git commit -m "feat(scroll): implement ScrollContent for StatusBar"
```

---

## Task 5: New `Text` widget with `Component` + `ScrollContent` impls

**Files:**
- Create: `src/widgets/text.rs`
- Modify: `src/widgets/mod.rs`
- Modify: `src/lib.rs` (re-export — optional; widgets aren't re-exported at top level currently, so skip)

- [ ] **Step 1: Write the failing test**

Create `src/widgets/text.rs` with test module first (TDD order — the type/methods won't exist yet):

```rust
//! Read-only text block. Wraps `ratatui::widgets::Paragraph`. Usable as
//! scrollable content inside `ScrollView`.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui::Frame;

pub struct Text {
    content: String,
    wrap: bool,
    alignment: Alignment,
}

impl Text {
    pub fn new(s: impl Into<String>) -> Self {
        Self { content: s.into(), wrap: true, alignment: Alignment::Left }
    }
    pub fn no_wrap(mut self) -> Self { self.wrap = false; self }
    pub fn alignment(mut self, a: Alignment) -> Self { self.alignment = a; self }

    fn build_paragraph(&self) -> Paragraph<'_> {
        let mut p = Paragraph::new(self.content.as_str()).alignment(self.alignment);
        if self.wrap { p = p.wrap(Wrap { trim: false }); }
        p
    }
}

impl Component for Text {
    fn handle_event(&mut self, _: &Event, _: &mut Context) -> Action { Action::Ignored }
    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &RenderContext) {
        frame.render_widget(self.build_paragraph(), area);
    }
    fn is_focusable(&self) -> bool { false }
    fn name(&self) -> &'static str { "Text" }
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

impl ScrollContent for Text {
    fn measure(&self, width: u16) -> u16 {
        if width == 0 { return 0; }
        self.build_paragraph().line_count(width) as u16
    }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, _ctx: &RenderContext) {
        self.build_paragraph().render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rctx<'a>(theme: &'a crate::theme::Theme) -> RenderContext<'a> {
        RenderContext::new(theme)
    }

    #[test]
    fn measure_single_line_unwrapped() {
        let t = Text::new("hello world");
        assert_eq!(t.measure(20), 1);
    }

    #[test]
    fn measure_wraps_at_width() {
        let t = Text::new("hello world");
        // "hello " + "world" wraps at width 6 into 2 lines. At width 5,
        // "hello" + "world" also → 2 lines (word-boundary wrap).
        assert_eq!(t.measure(5), 2);
    }

    #[test]
    fn measure_multiline_literal() {
        let t = Text::new("a\nb\nc");
        assert_eq!(t.measure(20), 3);
    }

    #[test]
    fn measure_width_zero_returns_zero() {
        let t = Text::new("hi");
        assert_eq!(t.measure(0), 0);
    }

    #[test]
    fn render_buf_writes_content() {
        let t = Text::new("hi");
        let theme = crate::theme::Theme::dark();
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        t.render_buf(&mut buf, area, &rctx(&theme));
        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(1, 0)].symbol(), "i");
    }

    #[test]
    fn is_not_focusable() {
        let t = Text::new("x");
        assert!(!t.is_focusable());
    }
}
```

- [ ] **Step 2: Wire module into `widgets/mod.rs`**

Append to `src/widgets/mod.rs`:

```rust
pub mod text;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --lib widgets::text::tests`
Expected: all 6 tests pass.

(Note: this task uses a "write the code and tests together" variant of TDD because the whole file is new. The tests drive the shape of `Text`'s API; running them verifies the implementation.)

- [ ] **Step 4: Commit**

```bash
git add src/widgets/text.rs src/widgets/mod.rs
git commit -m "feat(widgets): add Text widget with ScrollContent impl"
```

---

## Task 6: `ScrollContent` impl for `VStack`

**Files:**
- Modify: `src/containers/vstack.rs`

- [ ] **Step 1: Write the failing test**

In `src/containers/vstack.rs`, add to the existing `#[cfg(test)] mod tests`:

```rust
    use crate::scroll_content::ScrollContent;
    use crate::widgets::text::Text;
    use ratatui::buffer::Buffer;

    #[test]
    fn measure_sums_children_without_spacing() {
        let stack = VStack::new()
            .add(Box::new(Text::new("a")))
            .add(Box::new(Text::new("b")))
            .add(Box::new(Text::new("c")));
        assert_eq!(stack.measure(20), 3);
    }

    #[test]
    fn measure_sums_children_with_spacing() {
        let stack = VStack::new().spacing(1)
            .add(Box::new(Text::new("a")))
            .add(Box::new(Text::new("b")))
            .add(Box::new(Text::new("c")));
        // 3 text lines + 2 spacing gaps = 5
        assert_eq!(stack.measure(20), 5);
    }

    #[test]
    fn measure_includes_wrapped_child() {
        let stack = VStack::new()
            .add(Box::new(Text::new("hello world")))
            .add(Box::new(Text::new("x")));
        // "hello world" at width 5 → 2 lines. Plus "x" → 1. Total 3.
        assert_eq!(stack.measure(5), 3);
    }

    #[test]
    fn render_buf_stacks_children_vertically() {
        let stack = VStack::new()
            .add(Box::new(Text::new("aa")))
            .add(Box::new(Text::new("bb")));
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);
        stack.render_buf(&mut buf, area, &rctx);
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(0, 1)].symbol(), "b");
    }

    #[test]
    fn render_buf_skips_non_scroll_content_child() {
        // Leaf (from existing test setup) does not impl ScrollContent;
        // its row should render as blank (default cell).
        let stack = VStack::new()
            .add(Box::new(Leaf))                    // no ScrollContent
            .add(Box::new(Text::new("bb")));        // row 1: "bb"
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);
        stack.render_buf(&mut buf, area, &rctx);
        // Row 0 is Leaf → blank cells
        assert_eq!(buf[(0, 0)].symbol(), " ");
        // Row 1 is Text("bb")
        assert_eq!(buf[(0, 1)].symbol(), "b");
        assert_eq!(buf[(1, 1)].symbol(), "b");
    }

    #[test]
    fn as_scroll_content_returns_self() {
        let stack = VStack::new();
        assert!(stack.as_scroll_content().is_some());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib containers::vstack::tests`
Expected: FAIL — no `measure`, no `render_buf`, no `as_scroll_content`.

- [ ] **Step 3: Add `ScrollContent` impl and `as_scroll_content` override**

In `src/containers/vstack.rs`, add near the top imports:

```rust
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
```

Inside the existing `impl Component for VStack` block, add:

```rust
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
```

Add after the `impl Component for VStack` block:

```rust
impl ScrollContent for VStack {
    fn measure(&self, width: u16) -> u16 {
        let n = self.children.len() as u16;
        if n == 0 { return 0; }
        let total_spacing = self.spacing.saturating_mul(n.saturating_sub(1));
        let children_sum: u16 = self
            .children
            .iter()
            .map(|c| c.as_scroll_content().map(|sc| sc.measure(width)).unwrap_or(0))
            .sum();
        children_sum.saturating_add(total_spacing)
    }

    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        if self.children.is_empty() || area.height == 0 { return; }
        let mut y = area.y;
        let max_y = area.y.saturating_add(area.height);
        for child in &self.children {
            if y >= max_y { break; }
            let h = child.as_scroll_content().map(|sc| sc.measure(area.width)).unwrap_or(0);
            if h > 0 {
                let remaining = max_y.saturating_sub(y);
                let draw_h = h.min(remaining);
                let rect = Rect { x: area.x, y, width: area.width, height: draw_h };
                if let Some(sc) = child.as_scroll_content() {
                    sc.render_buf(buf, rect, ctx);
                }
                y = y.saturating_add(draw_h);
            }
            y = y.saturating_add(self.spacing);
        }
    }
}
```

Note the `render_buf` logic: if a child has no `ScrollContent`, its measured height is 0, so no rows are reserved for it — the stack collapses around it. This is a deliberate simplification for v0.2. If we later want to reserve a row and render it blank, we'd change `unwrap_or(0)` to some reserved height — not needed now.

- [ ] **Step 4: Update the non-ScrollContent-child test for new semantics**

Replace `render_buf_skips_non_scroll_content_child` above with:

```rust
    #[test]
    fn render_buf_collapses_non_scroll_content_child() {
        // Leaf does not impl ScrollContent; VStack treats its height as 0,
        // so subsequent children render in the rows that would have been
        // occupied.
        let stack = VStack::new()
            .add(Box::new(Leaf))                   // height 0 → collapses
            .add(Box::new(Text::new("bb")));       // renders at row 0
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);
        stack.render_buf(&mut buf, area, &rctx);
        assert_eq!(buf[(0, 0)].symbol(), "b");
        assert_eq!(buf[(1, 0)].symbol(), "b");
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib containers::vstack::tests`
Expected: all VStack tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/containers/vstack.rs
git commit -m "feat(scroll): implement ScrollContent for VStack"
```

---

## Task 7: `ScrollContent` impl for `HStack`

**Files:**
- Modify: `src/containers/hstack.rs`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` block at the bottom of `src/containers/hstack.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use crate::widgets::text::Text;
    use ratatui::buffer::Buffer;

    #[test]
    fn measure_returns_max_child_height() {
        // Two children given equal-width slices of the HStack width.
        // width=20, 2 children → each gets ~10 cols.
        // "aa" measures 1. "bb bb bb bb bb" at width 10 wraps to 3 lines.
        let stack = HStack::new()
            .add(Box::new(Text::new("aa")))
            .add(Box::new(Text::new("bb bb bb bb bb")));
        assert_eq!(stack.measure(20), 3);
    }

    #[test]
    fn measure_empty_is_zero() {
        assert_eq!(HStack::new().measure(20), 0);
    }

    #[test]
    fn render_buf_writes_children_side_by_side() {
        let stack = HStack::new()
            .add(Box::new(Text::new("aa")))
            .add(Box::new(Text::new("bb")));
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);
        stack.render_buf(&mut buf, area, &rctx);
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(1, 0)].symbol(), "a");
        assert_eq!(buf[(2, 0)].symbol(), "b");
        assert_eq!(buf[(3, 0)].symbol(), "b");
    }

    #[test]
    fn as_scroll_content_returns_self() {
        assert!(HStack::new().as_scroll_content().is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib containers::hstack::tests`
Expected: FAIL.

- [ ] **Step 3: Add `ScrollContent` impl**

In `src/containers/hstack.rs`, add near the top imports:

```rust
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
```

Inside the existing `impl Component for HStack` block, add:

```rust
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
```

Add after the `impl Component for HStack` block:

```rust
impl ScrollContent for HStack {
    fn measure(&self, width: u16) -> u16 {
        let n = self.children.len() as u16;
        if n == 0 || width == 0 { return 0; }
        let total_spacing = self.spacing.saturating_mul(n.saturating_sub(1));
        let usable = width.saturating_sub(total_spacing);
        let per_child = usable / n.max(1);
        let remainder = usable % n.max(1);
        self.children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                let extra = if (i as u16) < remainder { 1 } else { 0 };
                let child_w = per_child + extra;
                child.as_scroll_content().map(|sc| sc.measure(child_w)).unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }

    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        let n = self.children.len() as u16;
        if n == 0 || area.width == 0 { return; }
        let total_spacing = self.spacing.saturating_mul(n.saturating_sub(1));
        let usable = area.width.saturating_sub(total_spacing);
        let per_child = usable / n.max(1);
        let remainder = usable % n.max(1);
        let mut x = area.x;
        for (i, child) in self.children.iter().enumerate() {
            let extra = if (i as u16) < remainder { 1 } else { 0 };
            let w = per_child + extra;
            if w == 0 { continue; }
            let rect = Rect { x, y: area.y, width: w, height: area.height };
            if let Some(sc) = child.as_scroll_content() {
                sc.render_buf(buf, rect, ctx);
            }
            x = x.saturating_add(w).saturating_add(self.spacing);
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib containers::hstack::tests`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/containers/hstack.rs
git commit -m "feat(scroll): implement ScrollContent for HStack"
```

---

## Task 8: `ScrollContent` impl for `Grid`

**Files:**
- Modify: `src/containers/grid.rs`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` block at the bottom of `src/containers/grid.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use crate::widgets::text::Text;
    use ratatui::buffer::Buffer;

    #[test]
    fn measure_sums_row_heights() {
        // 2x2 grid; each cell a Text of height 1.
        let g = Grid::new(2, 2)
            .set(0, 0, Box::new(Text::new("a")))
            .set(0, 1, Box::new(Text::new("b")))
            .set(1, 0, Box::new(Text::new("c")))
            .set(1, 1, Box::new(Text::new("d")));
        assert_eq!(g.measure(10), 2);
    }

    #[test]
    fn measure_takes_max_in_row() {
        // Row 0: cell (0,0) height 1, cell (0,1) height 2 → row 0 = 2.
        // Row 1: one cell of height 1 → row 1 = 1.
        // Total = 3.
        let g = Grid::new(2, 2)
            .set(0, 0, Box::new(Text::new("a")))
            .set(0, 1, Box::new(Text::new("b\nb")))
            .set(1, 0, Box::new(Text::new("c")));
        assert_eq!(g.measure(20), 3);
    }

    #[test]
    fn measure_empty_is_zero() {
        let g = Grid::new(0, 0);
        assert_eq!(g.measure(10), 0);
    }

    #[test]
    fn render_buf_places_cells() {
        let g = Grid::new(1, 2)
            .set(0, 0, Box::new(Text::new("a")))
            .set(0, 1, Box::new(Text::new("b")));
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        g.render_buf(&mut buf, area, &rctx);
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(1, 0)].symbol(), "b");
    }

    #[test]
    fn as_scroll_content_returns_self() {
        assert!(Grid::new(1, 1).as_scroll_content().is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib containers::grid::tests`
Expected: FAIL.

- [ ] **Step 3: Add `ScrollContent` impl**

In `src/containers/grid.rs`, add near the top imports:

```rust
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
```

Inside the existing `impl Component for Grid` block, add:

```rust
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
```

Add after the `impl Component for Grid` block:

```rust
impl ScrollContent for Grid {
    fn measure(&self, width: u16) -> u16 {
        if self.rows == 0 || self.cols == 0 { return 0; }
        let col_w = width / self.cols;
        let mut total: u16 = 0;
        for row in &self.cells {
            let row_max: u16 = row
                .iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(|c| c.as_scroll_content())
                        .map(|sc| sc.measure(col_w))
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0);
            total = total.saturating_add(row_max);
        }
        total
    }

    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        if self.rows == 0 || self.cols == 0 { return; }
        let col_w = area.width / self.cols;
        let mut y = area.y;
        for row in &self.cells {
            let row_h: u16 = row
                .iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(|c| c.as_scroll_content())
                        .map(|sc| sc.measure(col_w))
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0);
            for (c, cell) in row.iter().enumerate() {
                if let Some(comp) = cell {
                    if let Some(sc) = comp.as_scroll_content() {
                        let rect = Rect {
                            x: area.x + (c as u16) * col_w,
                            y,
                            width: col_w,
                            height: row_h,
                        };
                        sc.render_buf(buf, rect, ctx);
                    }
                }
            }
            y = y.saturating_add(row_h);
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib containers::grid::tests`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/containers/grid.rs
git commit -m "feat(scroll): implement ScrollContent for Grid"
```

---

## Task 9: Refactor `ScrollView` to use virtual buffer

**Files:**
- Modify: `src/containers/scroll_view.rs`

This task replaces the whole file. Existing v0.1 tests (`page_down_advances_by_ten`, `home_end_jump`) don't exist yet — the file has no test module today — so we add them as part of this task.

- [ ] **Step 1: Write the failing tests**

Replace `src/containers/scroll_view.rs` entirely. Use the following content — the `impl` blocks are stubbed so the test module compiles and we can verify the tests fail at the assertion level:

```rust
//! Scroll container. Renders its child into an off-screen scratch `Buffer`
//! at the child's natural size (via `ScrollContent::measure` +
//! `render_buf`), then copies the viewport-sized row-slice into the real
//! frame buffer.
//!
//! The child must implement `ScrollContent`. Editable widgets (TextField,
//! Dropdown in open state, etc.) intentionally do not implement it, so
//! attempting to put one directly inside a `ScrollView` is a compile error.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::{Event, MouseKind};
use crate::scroll_content::ScrollContent;
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::cell::RefCell;

pub struct ScrollView {
    child: Box<dyn ScrollContent>,
    pub scroll: u16,
    scratch: RefCell<Buffer>,
}

impl ScrollView {
    pub fn new(child: Box<dyn ScrollContent>) -> Self {
        Self {
            child,
            scroll: 0,
            scratch: RefCell::new(Buffer::empty(Rect::new(0, 0, 0, 0))),
        }
    }

    pub fn scroll_to(&mut self, y: u16) { self.scroll = y; }

    pub fn scroll_by(&mut self, delta: i32) {
        if delta >= 0 {
            self.scroll = self.scroll.saturating_add(delta as u16);
        } else {
            self.scroll = self.scroll.saturating_sub((-delta) as u16);
        }
    }
}

impl Component for ScrollView {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        match event {
            Event::Key(k) => match k.code {
                KeyCode::PageDown => { self.scroll = self.scroll.saturating_add(10); Action::Absorbed }
                KeyCode::PageUp => { self.scroll = self.scroll.saturating_sub(10); Action::Absorbed }
                KeyCode::Home => { self.scroll = 0; Action::Absorbed }
                KeyCode::End => { self.scroll = u16::MAX; Action::Absorbed }
                _ => self.child.handle_event(event, ctx),
            },
            Event::Mouse(m) => match m.kind {
                MouseKind::ScrollUp => { self.scroll = self.scroll.saturating_sub(3); Action::Absorbed }
                MouseKind::ScrollDown => { self.scroll = self.scroll.saturating_add(3); Action::Absorbed }
                _ => self.child.handle_event(event, ctx),
            },
            _ => self.child.handle_event(event, ctx),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        if area.width == 0 || area.height == 0 { return; }
        let content_h = self.child.measure(area.width);
        if content_h == 0 { return; }

        let clamped_scroll = self.scroll.min(content_h.saturating_sub(area.height));

        let scratch_area = Rect::new(0, 0, area.width, content_h);
        let mut scratch = self.scratch.borrow_mut();
        if scratch.area() != &scratch_area {
            scratch.resize(scratch_area);
        }
        scratch.reset();

        self.child.render_buf(&mut scratch, scratch_area, ctx);

        let dst = frame.buffer_mut();
        let visible_h = area.height.min(content_h.saturating_sub(clamped_scroll));
        for row in 0..visible_h {
            for col in 0..area.width {
                let src_cell = scratch[(col, clamped_scroll + row)].clone();
                dst[(area.x + col, area.y + row)] = src_cell;
            }
        }
    }

    fn name(&self) -> &'static str { "ScrollView" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::text::Text;
    use crate::containers::vstack::VStack;
    use crate::theme::Theme;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(c: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code: c, modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press, state: KeyEventState::NONE,
        })
    }

    fn render_to_terminal(sv: &ScrollView, width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        terminal.draw(|f| {
            let rctx = RenderContext::new(&theme);
            sv.render(f, f.area(), &rctx);
        }).unwrap();
        terminal
    }

    fn make_tall_vstack(lines: u16) -> Box<dyn ScrollContent> {
        let mut stack = VStack::new();
        for i in 0..lines {
            stack.push(Box::new(Text::new(format!("L{i:02}"))));
        }
        Box::new(stack)
    }

    #[test]
    fn page_down_advances_by_ten() {
        let mut sv = ScrollView::new(make_tall_vstack(50));
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        sv.handle_event(&key(KeyCode::PageDown), &mut c);
        assert_eq!(sv.scroll, 10);
    }

    #[test]
    fn home_jumps_to_top() {
        let mut sv = ScrollView::new(make_tall_vstack(50));
        sv.scroll = 20;
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        sv.handle_event(&key(KeyCode::Home), &mut c);
        assert_eq!(sv.scroll, 0);
    }

    #[test]
    fn end_jumps_to_bottom_on_render() {
        let mut sv = ScrollView::new(make_tall_vstack(50));
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        sv.handle_event(&key(KeyCode::End), &mut c);
        assert_eq!(sv.scroll, u16::MAX);
        // After render, the actual viewport clamps what's shown.
        let term = render_to_terminal(&sv, 4, 5);
        let buf = term.backend().buffer();
        // Bottom 5 rows are L45..L49; top row shown is L45.
        let row0: String = (0..4).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(row0, "L45 ");
        let row4: String = (0..4).map(|x| buf[(x, 4)].symbol().to_string()).collect();
        assert_eq!(row4, "L49 ");
    }

    #[test]
    fn measures_child_and_copies_visible_slice() {
        let mut sv = ScrollView::new(make_tall_vstack(20));
        sv.scroll = 3;
        let term = render_to_terminal(&sv, 4, 4);
        let buf = term.backend().buffer();
        // Rows 3..7 of content should be visible: L03, L04, L05, L06.
        for (row_idx, expected) in ["L03 ", "L04 ", "L05 ", "L06 "].iter().enumerate() {
            let row: String = (0..4)
                .map(|x| buf[(x, row_idx as u16)].symbol().to_string())
                .collect();
            assert_eq!(&row, expected, "row {row_idx}");
        }
    }

    #[test]
    fn empty_rect_does_not_panic() {
        let sv = ScrollView::new(make_tall_vstack(10));
        let backend = TestBackend::new(10, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::dark();
        terminal.draw(|f| {
            let rctx = RenderContext::new(&theme);
            sv.render(f, Rect::new(0, 0, 0, 0), &rctx);
        }).unwrap();
    }

    #[test]
    fn grandchild_without_scroll_content_collapses() {
        // VStack containing a TextField (no ScrollContent impl) inside the
        // ScrollView. The TextField's rows don't exist (VStack collapses
        // around it), so the other child renders at row 0.
        use crate::widgets::text_field::TextField;
        let stack = VStack::new()
            .add(Box::new(TextField::new("name", "")))
            .add(Box::new(Text::new("hi")));
        let sv = ScrollView::new(Box::new(stack));
        let term = render_to_terminal(&sv, 4, 2);
        let buf = term.backend().buffer();
        let row0: String = (0..4).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(row0.trim(), "hi");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --lib containers::scroll_view::tests`
Expected: all 6 ScrollView tests pass.

(Because Task 9 writes both the implementation and the tests together, there is no "failing → passing" transition — the implementation must be correct on the first run. If any test fails, fix the implementation inline before committing.)

- [ ] **Step 3: Run the full test suite**

Run: `cargo test --lib`
Expected: all tests pass. Count should be substantially higher than the v0.1 39. Note any regressions.

- [ ] **Step 4: Commit**

```bash
git add src/containers/scroll_view.rs
git commit -m "feat(scroll): virtual-buffer ScrollView with ScrollContent child"
```

---

## Task 10: Update SPEC.md

**Files:**
- Modify: `SPEC.md`

- [ ] **Step 1: Remove ScrollView from "Known v0.1 limitations"**

In `SPEC.md`, the "Known v0.1 limitations" list starts at line 480. Delete item 1 (the "ScrollView does not clip" bullet) and renumber the remaining items. The list should become 5 items (dropping was #1).

- [ ] **Step 2: Remove "Virtual-buffer ScrollView" from v0.2 roadmap**

In the "v0.2 roadmap" list (starting at line 470), delete the `**Virtual-buffer ScrollView** — ...` bullet.

- [ ] **Step 3: Update the ScrollView widget catalog entry**

Replace the existing `### ScrollView` entry (around line 373) with:

```markdown
### ScrollView
```rust
ScrollView::new(Box::new(long_content))
```
- Wraps a child that implements `ScrollContent`; clips oversized content to the viewport via an internal scratch buffer
- Handles PgUp / PgDn / Home / End / mouse wheel
- Direct children **must** implement `ScrollContent` (compile-time check). Editable widgets (TextField, IntField, DateField, Dropdown when open, Calendar) intentionally do not — wrapping one in `ScrollView` is a compile error.
- Grandchildren (children of a VStack inside a ScrollView, etc.) are handled via `Component::as_scroll_content`; non-ScrollContent grandchildren have measured height 0 and their slot collapses. Documented limitation for v0.2; v0.3+ may lift this.
- Horizontal scrolling is not supported in v0.2 (content reflows to viewport width via `measure`).
```

- [ ] **Step 4: Add `Text` to the widget catalog**

Add a new entry after the `StatusBar` section, before `List`:

```markdown
### Text
```rust
Text::new("Some read-only content that may wrap.")
    .alignment(ratatui::layout::Alignment::Left)
```
- Read-only multi-line text. Wraps `ratatui::widgets::Paragraph`.
- Implements `ScrollContent` — primary widget for scrollable text blocks.
- `.no_wrap()` disables word-wrap; `.alignment(...)` sets alignment.
- Rich styled runs are deferred to a future `RichText` widget.
```

- [ ] **Step 5: Add a "ScrollContent" subsection under "Core types"**

After the `FocusManager` section (around line 218), add:

```markdown
### `ScrollContent` (src/scroll_content.rs)

```rust
pub trait ScrollContent: Component {
    fn measure(&self, width: u16) -> u16;
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext);
}
```

Implemented by widgets that are safe to render into an off-screen buffer (no cursor, no open overlays). Required by `ScrollView` for its direct child. The `Component` trait has a default `as_scroll_content(&self) -> Option<&dyn ScrollContent>` that returns `None`; widgets implementing `ScrollContent` override it to `Some(self)` so containers can delegate through `Box<dyn Component>` without a downcast.

v0.2 ships `ScrollContent` for: `Text`, `VStack`, `HStack`, `Grid`, `Toggle`, `Radio`, `StatusBar`. Editable widgets (`TextField`, `IntField`, `FloatField`, `DollarField`, `DateField`, `Calendar`) and overlay widgets (`Dropdown`) intentionally do not implement it.
```

- [ ] **Step 6: Update the "Tests you can run right now" count**

Near the bottom of SPEC.md (around line 506), replace `39 tests` with the new total. Run `cargo test --lib 2>&1 | grep 'test result'` to get the count, then update the number.

- [ ] **Step 7: Bump the section "Current coverage"**

Around line 446, the sentence "Current coverage: 39 tests across …" — update to the new count and add the new modules (`text`, `scroll_view`, `scroll_content`, plus the containers that gained tests).

- [ ] **Step 8: Add forward-compat note to design decisions**

In the "Design decisions worth knowing" section, add a new subsection:

```markdown
### Why `ScrollContent` is a sub-trait, not a method on Component

Making `ScrollView::new` take `Box<dyn ScrollContent>` (instead of `Box<dyn Component>`) gives us a compile-time check that the direct child is scroll-safe. Editable widgets (which can't render correctly into a buffer without cursor support) literally cannot compile inside a `ScrollView`. Adding the methods directly to `Component` with default `panic!()` or no-op implementations would push that check to runtime or produce silent rendering bugs.

v0.3+ can add `fn cursor_hint(&self) -> Option<Position> { None }` to `ScrollContent` to make editable widgets work inside a ScrollView, without any breaking change to the v0.2 API.
```

- [ ] **Step 9: Commit**

```bash
git add SPEC.md
git commit -m "docs: update SPEC for v0.2 ScrollView + Text + ScrollContent"
```

---

## Task 11: Final verification and cleanup pass

**Files:**
- None (verification only)

- [ ] **Step 1: Verify the full test suite**

Run: `cargo test`
Expected: 0 failures.

- [ ] **Step 2: Verify clean build**

Run: `cargo build`
Expected: 0 warnings.

- [ ] **Step 3: Verify clippy is clean (optional but recommended)**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings. If ratatui-version-specific clippy notes appear (e.g. deprecation of `Buffer::area()`), follow the suggestions.

- [ ] **Step 4: Verify the compile-error behavior manually**

Create a scratch file `/tmp/scroll_compile_fail.rs` with:

```rust
use tuile::{ScrollContent};
use tuile::containers::scroll_view::ScrollView;
use tuile::widgets::text_field::TextField;

fn main() {
    let _ = ScrollView::new(Box::new(TextField::new("x", "")));
}
```

Run: `rustc --edition 2024 --extern tuile=target/debug/libtuile.rlib -L target/debug/deps /tmp/scroll_compile_fail.rs -o /tmp/scroll_compile_fail 2>&1 | head -5`
Expected: compile error mentioning `the trait bound ... ScrollContent ... is not satisfied`.

(This is a one-time manual sanity check — no automated `trybuild` coverage in v0.2 per the spec.)

- [ ] **Step 5: Announce done**

No commit; this task is verification only. If everything passes, the v0.2 ScrollView work is complete.

---

## Plan self-review

**Spec coverage:**
- Virtual-buffer clipping for vertical scrolling → Task 9.
- `ScrollContent` sub-trait → Task 1.
- `measure(width) -> u16` → Task 1 (trait), Tasks 2–8 (impls).
- New `Text` widget → Task 5.
- `ScrollContent` impls for VStack, HStack, Grid, Toggle, Radio, StatusBar, Text → Tasks 2, 3, 4, 5, 6, 7, 8.
- `Component::as_scroll_content` default + overrides → Task 1 (default), Tasks 2–8 (overrides).
- Breaking change to `ScrollView::new` signature + removal of `set_content_height` → Task 9.
- `RefCell<Buffer>` scratch → Task 9.
- SPEC.md updates (remove limitation, remove v0.2 item, add Text, document ScrollContent) → Task 10.
- No `trybuild` → confirmed in Task 11 Step 4 (manual check only).

**Type consistency check:**
- `ScrollContent::measure` signature matches across trait definition (Task 1) and all impls (Tasks 2–8).
- `ScrollContent::render_buf` signature matches: `(&self, &mut Buffer, Rect, &RenderContext)` everywhere.
- `as_scroll_content` returns `Option<&dyn ScrollContent>` everywhere.
- `ScrollView::new` takes `Box<dyn ScrollContent>` consistently with the spec.

**Placeholder scan:** none found. All code blocks contain the actual code to write.
