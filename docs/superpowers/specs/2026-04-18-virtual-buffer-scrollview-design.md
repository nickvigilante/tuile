# Design: virtual-buffer ScrollView

Status: proposed (v0.2)
Date: 2026-04-18

## Motivation

`ScrollView` in v0.1 tracks a `scroll` offset and a manually-supplied `content_height` but doesn't use them at render time — it passes its viewport rect straight through to the child. The child sizes itself to the viewport, so there is nothing extra to scroll through and no clipping takes place. The fields are dead weight.

This spec defines the v0.2 work to make `ScrollView` actually scroll, by rendering the child into an off-screen scratch buffer at the child's natural size and blitting a viewport-sized window onto the screen ("virtual buffer").

## Scope

In scope for v0.2:

- Virtual-buffer clipping for vertical scrolling.
- A new `ScrollContent` sub-trait that `ScrollView`'s child must implement.
- A `measure(width) -> u16` method on `ScrollContent` for self-describing natural height.
- A new `Text` widget (wraps `ratatui::widgets::Paragraph`) as the canonical read-only scrollable content.
- `ScrollContent` impls for `VStack`, `HStack`, `Grid`, `Toggle`, `Radio`, `StatusBar`, and `Text`.

Explicitly out of scope for v0.2:

- Horizontal scrolling. (Width-driven reflow of content — e.g., Text wrapping at the viewport width — is in scope, since it comes free with `measure(width)`.)
- Editable widgets inside a scroll (TextField, IntField, DateField, Calendar, Dropdown). These don't implement `ScrollContent` and will fail to compile if wrapped in a `ScrollView`.
- Cursor-hint plumbing for v0.3+ editable-in-scroll. The design leaves room for it (see "Forward compatibility") but does not implement it.
- `trybuild`-based compile-fail tests. Deferred — document the expected error in the doc comment instead.
- Nested ScrollViews, Modals, Tabs, etc. inside a ScrollView.

## Design

### New trait: `ScrollContent`

```rust
pub trait ScrollContent: Component {
    /// Natural height the widget needs to fully render at the given width.
    /// Recursive for containers (a VStack sums its children's measures).
    fn measure(&self, width: u16) -> u16;

    /// Render into a buffer at the given area. No cursor support —
    /// buffer rendering does not have access to `Frame::set_cursor_position`.
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext);
}
```

`ScrollContent` extends `Component`. A widget that is safe to put inside a `ScrollView` implements both.

Rationale for a sub-trait instead of adding methods to `Component`:

- Direct children of `ScrollView` are checked at compile time. Writing `ScrollView::new(Box::new(TextField::new(...)))` produces `the trait ScrollContent is not implemented for TextField` — a hard compile error, not a runtime panic or a docs-only warning.
- Leaf widgets like `TextField` stay clean; they don't grow methods that would panic if called.

### `ScrollView` refactor

```rust
pub struct ScrollView {
    child: Box<dyn ScrollContent>,   // was: Box<dyn Component>
    pub scroll: u16,
    scratch: RefCell<Buffer>,        // reused across frames; resized as needed
}

impl ScrollView {
    pub fn new(child: Box<dyn ScrollContent>) -> Self;
    pub fn scroll_to(&mut self, y: u16);
    pub fn scroll_by(&mut self, delta: i32);
}
```

The `content_height` field and `set_content_height` method are removed — they're superseded by `child.measure(width)`.

Render algorithm:

1. `content_h = self.child.measure(area.width)`.
2. Clamp `self.scroll` into `[0, content_h.saturating_sub(area.height)]`.
3. Resize the scratch `Buffer` to `(area.width, content_h)` if not already that size; reset cells to default.
4. `self.child.render_buf(&mut scratch, Rect::new(0, 0, area.width, content_h), ctx)`.
5. Copy rows `[scroll .. scroll + area.height]` from scratch into `frame.buffer_mut()` at the viewport rect.

Event handling is unchanged from v0.1: `PgUp` / `PgDn` / `Home` / `End` / mouse wheel adjust `scroll`; other events forward to the child.

Why `RefCell<Buffer>`: `Component::render` takes `&self`. The scratch buffer mutates every frame (resize + fill). `RefCell` is the minimal fix; tuile is single-threaded so there is no sync concern. The alternative (hoisting scratch storage outside the component) is worse ergonomically and leaks implementation detail.

### `ScrollContent` implementations shipped in v0.2

| Type | `measure` | `render_buf` |
|---|---|---|
| `Text` (new) | wrapped-line count at width W | `ratatui::widgets::Paragraph` |
| `VStack` | Σ of children's `measure(width)` + spacing (each child is given the full width) | iterate children vertically |
| `HStack` | max of children's `measure(child_width)` (each child gets an equal slice of the width, matching v0.1 HStack layout) | iterate children horizontally |
| `Grid` | Σ of row heights, where each row height = max of `cell.measure(column_width)` across cells in that row (each column gets an equal slice of the width) | iterate cells |
| `Toggle` | 1 | underlying ratatui render into buffer |
| `Radio` | 1 | underlying ratatui render into buffer |
| `StatusBar` | 1 | underlying ratatui render into buffer |

Explicitly NOT implementing `ScrollContent` in v0.2:

- `TextField`, `IntField`, `FloatField`, `DollarField`, `DateField`, `Calendar` — editable, cursor-bearing.
- `Dropdown` — has an open-state overlay that would not compose with buffer-based rendering.
- `List`, `Table` — manage their own scrolling natively; wrapping them in a ScrollView would duplicate behavior.
- `Modal`, `Tabs`, `Form`, `Overlay`, `ScrollView` — top-level layout types, not content.

### Grandchildren: the `as_scroll_content` escape hatch

Container types (`VStack`, etc.) store `Vec<Box<dyn Component>>`, not `Vec<Box<dyn ScrollContent>>`. We do not want to change that — cascading the stronger bound through every container would make the API hostile for the common non-scrolled cases.

Instead, add one optional method to `Component`:

```rust
pub trait Component {
    // ...existing methods...

    /// If this component also implements ScrollContent, return Some(self).
    /// Default: None. Widgets that implement ScrollContent override this.
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { None }
}
```

Widgets that implement `ScrollContent` override to `Some(self)`. When a container (e.g., VStack) is asked to `render_buf`, it iterates children and calls `child.as_scroll_content()` on each; if `Some`, it delegates to that child's `render_buf`; if `None`, it leaves the child's row range blank. (Logging behavior is not specified here; tuile has no logger integration in v0.2.)

Net effect:

- **Direct children of `ScrollView`** — checked at compile time (trait bound).
- **Grandchildren of `ScrollView`** — checked at runtime; non-`ScrollContent` widgets render as blank. Documented as a known limitation.

Putting a `TextField` inside a `VStack` inside a `ScrollView` compiles, but the TextField renders as blank space. This is acceptable for v0.2 because the intended use case is read-only content; the compile error on direct children catches the most common mistake.

### New widget: `Text`

```rust
// src/widgets/text.rs
pub struct Text {
    content: String,
    wrap: bool,            // default true
    alignment: Alignment,  // default Left
}

impl Text {
    pub fn new(s: impl Into<String>) -> Self;
    pub fn no_wrap(self) -> Self;
    pub fn alignment(self, a: Alignment) -> Self;
}

impl Component for Text { /* ... */ }
impl ScrollContent for Text {
    fn measure(&self, width: u16) -> u16 {
        // wrapped line count at the given width
    }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        // build a ratatui::widgets::Paragraph and call Widget::render(buf, area)
    }
}
```

`Text` is `is_focusable() -> false`. `RichText` (styled runs, inline color) is explicitly deferred to v0.3+.

### Forward compatibility: editable-in-scroll (v0.3+)

Not implemented here, but the design does not close the door. When v0.3+ adds editable-in-scroll support, we expect to add an optional cursor-hint method on `ScrollContent`:

```rust
fn cursor_hint(&self) -> Option<Position> { None }  // in content-local coords
```

`ScrollView` would translate the hint from content-space to viewport-space and call `frame.set_cursor_position` in its own `render`. Nothing in this v0.2 design prevents that extension.

## Testing

Following SPEC.md convention (behavior-tested, not snapshot-tested):

**`scroll_view::tests`:**
- `clamps_scroll_to_content_height` — set scroll past content; render; assert `self.scroll` is clamped.
- `measures_child_and_sizes_buffer` — child of known measure; verify viewport copies correct rows for a given scroll.
- `page_down_advances_by_ten` — keep existing.
- `home_end_jump` — keep existing.
- `empty_rect_noop` — viewport height 0; render does not panic.
- `grandchild_without_scroll_content_renders_blank` — VStack containing a TextField inside a ScrollView; the TextField's row range in the buffer is blank, not crashed.

**`text::tests` (new module):**
- `measure_wraps_at_width` — `"hello world"` at width 5 → measures 3 lines.
- `render_buf_writes_content` — render and read back specific cells.

**`vstack::tests` addition:**
- `measure_sums_children` — three `Text` widgets of known height → VStack measure returns the sum (+ spacing).

Compile-fail test for the sub-trait bound is deferred (see "Scope").

## Breaking changes

This is a breaking change to the `ScrollView` API:

- `ScrollView::new` now takes `Box<dyn ScrollContent>` instead of `Box<dyn Component>`.
- `ScrollView::set_content_height` is removed.
- `ScrollView::content_height` field is removed.

tuile is v0.1 ("API unstable" per SPEC.md:3). The sole known consumer is `ynab-budget-manager`; any breakage there is acceptable and expected to be small (ScrollView is not used in that app's Schedule modal or Budget tab).

No deprecation shim. `Component` gains one additional default method (`as_scroll_content`) that is backward-compatible.

## SPEC.md updates (follow-up after implementation)

- Remove ScrollView from "Known v0.1 limitations" (#1).
- Remove "Virtual-buffer ScrollView" from the v0.2 roadmap.
- Add `Text` to the widget catalog.
- Update the ScrollView section to describe the new API, the no-editable-widgets constraint, and the `as_scroll_content` escape hatch.
- Note the forward-compat hook for v0.3+ editable-in-scroll.
