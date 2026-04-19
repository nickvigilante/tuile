# tuile specification

> Status: v0.2 — API unstable. This document describes the current shape of the crate and the design rationale behind it.

## What tuile is

tuile is a reusable TUI component framework built on [ratatui](https://github.com/ratatui-org/ratatui). It provides a Component trait, a focus manager, a cascading theme system, and a library of widgets and layout containers.

It was written because the existing TUI framework options in Rust have specific pain points:
- **ratatui alone** — widgets are stateless render helpers; you own all state and event routing. Great flexibility, zero abstractions, but you reinvent text inputs, dropdowns, focus, etc. for every app.
- **tui-realm** — provides a component model but with runtime-typed enum state, a three-hop event pipeline (`Event → Cmd → CmdResult → Msg`), and two traits per component (`MockComponent` + `Component`) with significant boilerplate.

tuile's bet: a lightweight component trait with concrete Rust types, one-hop event handling, and a CSS-like cascading theme hits a sweet spot between raw ratatui and a full framework.

## Naming

"tuile" is French for "tile" — also a thin curved French cookie. It contains "tui" naturally, mirroring how "ratatui" is derived from "ratatouille" + "tui." Both crates are French culinary words that happen to contain the substring "tui."

## Directory layout

```
tuile/
├── Cargo.toml
├── README.md
├── SPEC.md               (this file)
├── src/
│   ├── lib.rs            re-exports
│   ├── action.rs         Action enum
│   ├── component.rs      Component trait, RenderContext, Context
│   ├── event.rs          Event enum (keyboard / paste / mouse / resize / tick)
│   ├── focus.rs          FocusManager with scopes
│   ├── scroll_content.rs  ScrollContent trait
│   ├── theme/
│   │   ├── mod.rs        Theme presets (dark, light)
│   │   ├── tokens.rs     Theme struct with semantic tokens
│   │   └── contrast.rs   WCAG contrast ratio computation
│   ├── validation.rs     ValidationResult enum
│   ├── widgets/
│   │   ├── text_field.rs
│   │   ├── int_field.rs
│   │   ├── float_field.rs
│   │   ├── dollar_field.rs
│   │   ├── toggle.rs
│   │   ├── radio.rs
│   │   ├── dropdown.rs
│   │   ├── date_field.rs
│   │   ├── calendar.rs
│   │   ├── status_bar.rs
│   │   ├── text.rs
│   │   ├── list.rs
│   │   └── table.rs
│   └── containers/
│       ├── vstack.rs
│       ├── hstack.rs
│       ├── grid.rs
│       ├── overlay.rs
│       ├── scroll_view.rs
│       ├── form.rs
│       ├── modal.rs
│       └── tabs.rs
└── tests/
    └── integration.rs    (placeholder; integration tests will go here)
```

## Design principles

1. **Concrete types, not runtime enums.** A `TextField`'s state is `String`; a `Toggle`'s is `bool`. No `StateValue` enum with `unwrap_string()`, `unwrap_bool()` panics.
2. **One trait, not two.** Every widget and container implements `Component`. There's no `MockComponent`/`Component` split.
3. **Children opt-in.** The `Component` trait has default `children_mut() → Vec::new()`. Leaf widgets don't override it; containers do.
4. **One-hop events.** `handle_event(&Event) → Action`. No `Event → Cmd → CmdResult → Msg` pipeline.
5. **Cascading themes.** Semantic tokens (`primary`, `surface`, `error`, etc.) cascade through a `RenderContext`, CSS-style, with per-widget override support.
6. **Section 508 contrast by default.** Built-in `dark()` and `light()` themes are unit-tested to pass WCAG AA (≥4.5:1).
7. **Mouse-first-class.** The `Event` type includes mouse events; widgets handle wheel scrolling out of the box; `FocusManager` can move focus by click-to-rect.
8. **Composable, not framework-y.** Any widget works standalone. You don't have to buy into an `Application` struct — widgets can be dropped into an existing ratatui run loop.

## Core types

### `Event` (src/event.rs)

```rust
pub enum Event {
    Key(crossterm::event::KeyEvent),
    Paste(String),  // sanitized (\r and \n stripped)
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,  // synthetic, emitted by app on a clock
}

pub struct MouseEvent {
    pub kind: MouseKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: crossterm::event::KeyModifiers,
}

pub enum MouseKind {
    Down(MouseButton), Up(MouseButton), Drag(MouseButton),
    Moved, ScrollUp, ScrollDown, ScrollLeft, ScrollRight,
}

pub enum MouseButton { Left, Right, Middle }

impl Event {
    pub fn from_crossterm(ev: crossterm::event::Event) -> Option<Self>;
}
```

The app's run loop reads `crossterm::event::Event`, converts via `Event::from_crossterm`, and dispatches to the root component's `handle_event`. A periodic timer emits `Event::Tick` (e.g., every 120 ms) for spinners, blinking cursors, and `StatusBar`'s auto-dismiss.

### `Action` (src/action.rs)

```rust
pub enum Action {
    Absorbed,             // handled, no observable change
    Changed,              // value changed
    Submit,               // Enter/confirm
    Cancel,               // Esc/cancel
    Custom(Box<dyn Any + Send>),  // typed escape hatch
    Ignored,              // unhandled; parent can try
}

impl Action {
    pub fn is_handled(&self) -> bool;  // !Ignored
}
```

Parents use `Ignored` vs `is_handled()` to decide whether to try their own handlers. `Changed` signals to the parent that a re-render or side effect may be warranted. `Submit`/`Cancel` are used by field widgets to signal the editing cycle (Enter commits, Esc reverts).

### `Component` trait (src/component.rs)

```rust
pub trait Component {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action;
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext);

    fn is_focusable(&self) -> bool { true }
    fn traps_focus(&self) -> bool { false }
    fn children_mut<'a>(&'a mut self) -> Vec<&'a mut (dyn Component + 'a)> { Vec::new() }
    fn children(&self) -> Vec<&dyn Component> { Vec::new() }
    fn name(&self) -> &'static str { "Component" }
}

pub struct Context<'a> { pub theme: &'a Theme }

pub struct RenderContext<'a> { pub theme: &'a Theme }
impl<'a> RenderContext<'a> {
    pub fn new(theme: &'a Theme) -> Self;
    pub fn with_theme<F, R>(&self, modify: impl FnOnce(&mut Theme), render: F) -> R
    where F: FnOnce(&RenderContext) -> R;
}
```

Notes:
- The explicit lifetime on `children_mut` is required because Rust's lifetime elision otherwise infers `'static` for bare trait objects returned from a method taking `&mut self`, and that breaks any container that wants to hand out borrows of its children. This is a Rust-language constraint, not a design choice.
- `traps_focus` is what Modal overrides to prevent Tab from escaping it.
- `is_focusable` is false for layout containers (VStack, Grid, Modal), true for widgets by default.

### `Theme` (src/theme/tokens.rs)

```rust
pub struct Theme {
    // Surfaces
    pub surface, surface_raised, surface_sunken: Color,
    // Content
    pub on_surface, on_surface_dim, on_surface_strong: Color,
    // Accent
    pub primary, on_primary, primary_dim: Color,
    // Secondary
    pub secondary, on_secondary: Color,
    // Semantic
    pub error, on_error, success, warning, info: Color,
    // Decoration
    pub border, border_focused, divider, cursor: Color,
}

impl Theme {
    pub fn dark() -> Self;   // tested to pass WCAG AA
    pub fn light() -> Self;  // tested to pass WCAG AA
    pub fn focused_style(&self) -> Style;
    pub fn unfocused_style(&self) -> Style;
    pub fn label_style(&self, focused: bool) -> Style;
    pub fn error_style(&self) -> Style;
    pub fn warning_style(&self) -> Style;
}

// src/theme/contrast.rs
pub fn contrast_ratio(fg: Color, bg: Color) -> Option<f64>;
pub fn meets_aa(fg: Color, bg: Color) -> bool;  // ≥ 4.5:1
```

Theme cascading uses `RenderContext::with_theme`: a parent clones its theme, mutates specific tokens, and renders a subtree with the modified theme in effect. This is how you'd render a subtree in "error mode" (e.g., `t.primary = t.error`) without each descendant needing explicit theming logic.

Contrast validation is a hard gate for the bundled presets. If you want to fork/extend a theme, call `meets_aa(fg, bg)` for each critical pair.

### `FocusManager` (src/focus.rs)

```rust
pub struct FocusId(u64);

pub struct FocusManager { /* ... */ }

impl FocusManager {
    pub fn new() -> Self;
    pub fn new_id(&mut self) -> FocusId;

    // Call at the start of each render pass.
    pub fn begin_frame(&mut self);

    // Register a focusable rendered this frame.
    pub fn register(&mut self, id: FocusId, rect: Rect);

    pub fn push_scope(&mut self);   // modal opens
    pub fn pop_scope(&mut self);    // modal closes

    pub fn focused(&self) -> Option<FocusId>;
    pub fn focus_next(&mut self);
    pub fn focus_prev(&mut self);
    pub fn focus_at(&mut self, column: u16, row: u16) -> bool;
}
```

**Scopes:** focus is a stack. The top scope is active; all lower scopes are suspended. Pushing a new scope (when a modal opens) traps focus inside it — Tab cycles only within the new scope. Pop restores the previous scope and its previously-focused item.

**Cross-frame preservation:** `begin_frame` clears the list of registered focusables but remembers the currently-focused id. During the render pass, widgets call `register(id, rect)`. If the previously-focused id re-registers, it's reselected; if it doesn't (e.g., the widget was removed), focus falls to the first registered item.

**Mouse focus:** `focus_at(col, row)` scans the active scope's rects and moves focus to whichever contains the point.

### `ScrollContent` (src/scroll_content.rs)

```rust
pub trait ScrollContent: Component {
    fn measure(&self, width: u16) -> u16;
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext);
}
```

Implemented by widgets that are safe to render into an off-screen buffer (no cursor, no open overlays). Required by `ScrollView` for its direct child. The `Component` trait has a default `as_scroll_content(&self) -> Option<&dyn ScrollContent>` that returns `None`; widgets implementing `ScrollContent` override it to `Some(self)` so containers can delegate through `Box<dyn Component>` without a downcast.

v0.2 ships `ScrollContent` for: `Text`, `VStack`, `HStack`, `Grid`, `Toggle`, `Radio`, `StatusBar`. Editable widgets (`TextField`, `IntField`, `FloatField`, `DollarField`, `DateField`, `Calendar`) and overlay widgets (`Dropdown`) intentionally do not implement it.

## Widget catalog

All field widgets (TextField, IntField, FloatField, DollarField, DateField) follow an **Enter-to-edit** model:
- Not editing: showing the value. Enter begins editing (cursor appears).
- Editing: keystrokes modify value. Enter commits (Submit action). Esc reverts (Cancel action).
- Dirty tracking: shows `•` marker when current value differs from last committed.
- Validation: `.validate() → ValidationResult` (Valid or Invalid(reason)).

### TextField
```rust
TextField::new("Name", "initial value")
    .required()
    .char_filter(|c| c.is_ascii_digit() || c == '.')
```
- Cursor within value (Left/Right/Home/End/Delete/Backspace)
- `char_filter` restricts which characters are accepted when typed
- `required()` fails validation on empty value

### IntField
```rust
IntField::new("Count", 5).range(0, 100).required()
```
- i64-only; Up/Down increments/decrements in editing mode
- Range clamping on increment; `range(min, max)` where `min >= 0` disables the `-` key

### FloatField
```rust
FloatField::new("Rate", 3.5).range(0.0, 100.0).decimals(2)
```
- f64 with configurable decimal precision (for `set_value` display)

### DollarField
```rust
DollarField::new("Amount", 150_500).required()
```
- Stores milliunits internally (×1000 per dollar — matches YNAB's API representation)
- Parses `150`, `150.00`, `$150`, `$150.00`, `-50.00`, `1,234.56`
- Rejects more than 2 decimal places

### Toggle
```rust
Toggle::new("Enabled", true)
```
- **Enter or Space flips. `t` does NOT.** (Explicit unit test enforces this.)
- Shows `◉ ON` (green) / `○ OFF` (gray) for clear visual distinction

### Radio
```rust
Radio::new("Weekend", vec!["None".into(), "Previous".into(), "Next".into()], 0)
```
- Horizontal layout, `◉` for selected, `○` for unselected
- Left/Right cycles; Enter commits; Esc reverts
- Intended for ≤5 options (for larger lists, use Dropdown)

### Dropdown
```rust
Dropdown::new("Category", options, Some(0))
    .required()
    .allow_create()
```
- Closed state: shows selected value + `▼`
- Open state: filter line + selectable list of matches, rendered as an overlay below the anchor rect
- Type to filter (case-insensitive substring match)
- `allow_create()` lets Enter on a no-match filter add a new option

### DateField
```rust
DateField::new("Anchor Date", Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()))
```
- Three sub-fields: YYYY / MM / DD (each an IntField internally)
- Left/Right moves between sub-fields; Up/Down increments the focused sub-field
- Enter commits the composed date (must parse to a valid NaiveDate)

### Calendar
```rust
Calendar::new(chrono::Local::now().naive_local().date())
```
- Month grid view with Mo-Su column headers
- Arrow keys navigate days; PgUp/PgDn changes month
- Enter submits; Esc reverts to committed value

### StatusBar
```rust
let mut bar = StatusBar::new();
bar.set("Saved");  // 45-tick countdown starts
```
- One-line message that auto-dismisses on `Event::Tick`
- Not focusable

### Text
```rust
Text::new("Some read-only content that may wrap.")
    .alignment(ratatui::layout::Alignment::Left)
```
- Read-only multi-line text. Wraps `ratatui::widgets::Paragraph`.
- Implements `ScrollContent` — primary widget for scrollable text blocks.
- `.no_wrap()` disables word-wrap; `.alignment(...)` sets alignment.
- Rich styled runs are deferred to a future `RichText` widget.

### List
```rust
let mut list = List::new(vec!["item 1".into(), "item 2".into()]);
```
- Scrollable single-select list
- Up/Down, PageUp/PageDown, Home/End, Enter (Submit), mouse wheel
- **Bottom-clamp invariant:** scroll never extends past content (no empty space below the last item)

### Table
```rust
Table::new(vec!["Name".into(), "Amount".into()],
           vec![vec!["Rent".into(), "$1200.00".into()]])
```
- Scrollable table with header row
- Same navigation keys as List
- `widths(Vec<Constraint>)` for column sizing

## Container catalog

Containers return `is_focusable() → false` by default. They own children and route events to them.

### VStack / HStack
```rust
VStack::new().spacing(1)
    .add(Box::new(field1))
    .add(Box::new(field2))
```
- Proportional layout: children share available height (VStack) or width (HStack) equally, plus optional spacing between them
- `handle_event` forwards to the first child that doesn't `Ignore`

### Grid
```rust
Grid::new(3, 2)
    .set(0, 0, Box::new(field1))
    .set(1, 1, Box::new(field2))
```
- Fixed rows × columns; cells empty unless `set`
- Children share row height and column width equally

### Overlay
```rust
let mut overlay = Overlay::new(Box::new(menu));
overlay.show(Rect { x: 10, y: 5, width: 30, height: 8 });
```
- Absolute-positioned layer rendered on top of other content
- `traps_focus()` when visible
- Used internally by (future) ContextMenu; Dropdown does its own overlay rendering for v0.1

### Modal
```rust
let mut modal = Modal::new("Edit Schedule", Box::new(form))
    .size_pct(70, 70);
modal.show();
```
- Centered overlay with titled, bordered frame
- `traps_focus()` when open
- Child is rendered inside the block's inner rect

### ScrollView
```rust
ScrollView::new(Box::new(long_content))
```
- Wraps a child that implements `ScrollContent`; clips oversized content to the viewport via an internal scratch buffer
- Handles PgUp / PgDn / Home / End / mouse wheel
- Direct children **must** implement `ScrollContent` (compile-time check). Editable widgets (TextField, IntField, DateField, Dropdown when open, Calendar) intentionally do not — wrapping one in `ScrollView` is a compile error.
- Grandchildren (children of a VStack inside a ScrollView, etc.) are handled via `Component::as_scroll_content`; non-ScrollContent grandchildren have measured height 0 and their slot collapses. Documented limitation for v0.2; v0.3+ may lift this.
- Horizontal scrolling is not supported in v0.2 (content reflows to viewport width via `measure`).

### Form
```rust
Form::new()
    .add(Box::new(name_field))
    .add(Box::new(amount_field))
```
- Vertical field list with Up/Down navigation between fields
- Enter begins editing the focused child (routes Enter to it); subsequent keys go to the editing child until it emits `Submit` or `Cancel`
- Eliminates shortcut conflicts: while a field is editing, typed letters go into the field, not to Form's shortcuts

### Tabs
```rust
Tabs::new()
    .add("Budget", Box::new(budget_panel))
    .add("Schedules", Box::new(schedule_panel))
```
- Tab bar on top + single active panel below
- Tab / Shift+Tab cycles active tab
- Events not consumed by the tab bar are forwarded to the active panel

## Using tuile in an app

Minimum wiring:

```rust
use tuile::{Theme, Event, Component, Context, RenderContext};
use tuile::widgets::text_field::TextField;

// 1. State: one or more components
let mut name = TextField::new("Name", "").required();

// 2. In your run loop:
//    - Read crossterm event
//    - Convert via Event::from_crossterm
//    - Dispatch: theme = Theme::dark(); let mut ctx = Context { theme: &theme };
//    - name.handle_event(&ev, &mut ctx)
//    - Draw: let rctx = RenderContext::new(&theme);
//    - terminal.draw(|f| name.render(f, f.area(), &rctx))
```

For a modal containing a form of several widgets, compose:

```rust
use tuile::containers::{form::Form, modal::Modal};

let form = Form::new()
    .add(Box::new(TextField::new("Name", "").required()))
    .add(Box::new(DollarField::new("Amount", 0).required()))
    .add(Box::new(Toggle::new("Enabled", true)));

let mut modal = Modal::new("Edit Schedule", Box::new(form));
modal.show();
```

Then route events/renders through `modal` — it traps focus, handles centering, and delegates to the form.

## Testing approach

Each widget/container module includes `#[cfg(test)]` unit tests that exercise:
- Key handling (Up/Down/Left/Right/Enter/Esc)
- Dirty tracking (changes → dirty; commit clears; revert clears)
- Validation edge cases (empty, out-of-range, bad format)
- Mode transitions (not-editing → editing → committed)

Rendering is deliberately **not** unit-tested — it's verified manually via the host app. Rendering tests in TUI frameworks tend to be brittle (snapshot tests for character grids) and expensive to maintain. Behavior is what matters.

Current coverage: 78 tests across Event, Theme (contrast + preset compliance), FocusManager, ScrollContent, ScrollView, Text, VStack, HStack, Grid, TextField, IntField, FloatField, DollarField, DateField, Calendar, Toggle, Radio, Dropdown, StatusBar, List, Table.

## Design decisions worth knowing

### Why `children_mut` has an explicit lifetime

The default `fn children_mut(&mut self) -> Vec<&mut dyn Component>` works for an empty default but breaks as soon as a container tries to override it, because Rust infers `'static` for the bare trait object. The fix: explicit lifetime `<'a>(&'a mut self) -> Vec<&'a mut (dyn Component + 'a)>`. This is a Rust-language artifact, not a tuile design choice.

### Why `render` is `&self` (not `&mut`)

ratatui's `Frame::render_widget` takes the widget by value, and the typical pattern is to hold a reference to your state and build the ratatui widget from it. Making `render` immutable forces widgets to compute any per-frame derived state (like scroll adjustment in List) inline inside `render`, using the stored state as a hint. Mutation belongs in `handle_event`, not `render`.

### Why themes clone on override

`RenderContext::with_theme` clones the theme and mutates the clone. Themes are small (<30 color fields) and typically overridden at modal/scope boundaries (not per-widget), so cloning is cheap. The alternative — a linked chain of overrides — adds complexity without meaningful performance upside.

### Why StatusBar has a tick field instead of timestamps

Tick-based timing ties the dismissal cadence to the host app's render clock. That clock is the thing the user actually perceives (frames), so "hide after 45 ticks" is more predictable than "hide after 5 seconds" when render latency varies.

### Why Dropdown renders its own overlay instead of using the Overlay container

Overlay is a general container meant for absolutely-positioned children (context menus, tooltips). Dropdown's overlay is tightly coupled to the closed-state anchor rect, so doing it inline in Dropdown's `render` avoids the need to hoist the filter/cursor state into the Overlay-wrapping parent. When we add ContextMenu in v0.2, we'll use Overlay proper.

### Why `ScrollContent` is a sub-trait, not a method on Component

Making `ScrollView::new` take `Box<dyn ScrollContent>` (instead of `Box<dyn Component>`) gives us a compile-time check that the direct child is scroll-safe. Editable widgets (which can't render correctly into a buffer without cursor support) literally cannot compile inside a `ScrollView`. Adding the methods directly to `Component` with default `panic!()` or no-op implementations would push that check to runtime or produce silent rendering bugs.

v0.3+ can add `fn cursor_hint(&self) -> Option<Position> { None }` to `ScrollContent` to make editable widgets work inside a ScrollView, without any breaking change to the v0.2 API.

## v0.2 roadmap

- **ContextMenu widget** — right-click menus, built on Overlay.
- **Subscriptions** — let non-focused components react to specific event patterns (analogous to tui-realm's Sub).
- **Ports** — integrate user-defined async event sources (background polls, API tickers) into the event loop.
- **Animations** — first-class spinner/fade/transition helpers.
- **Theme hot-reload** — load themes from TOML, reload without restart.
- **A11y hints** — structured labels for screen readers (where terminals support them).

## Known v0.2 limitations

1. **No keyboard-driven focus wiring inside Form.** Form has internal focus tracking, but it isn't wired into `FocusManager`. Top-level focus management (Tab/Shift+Tab across whole screens) is up to the app.
2. **Dropdown overlay positions below the anchor.** If the anchor is near the bottom of the screen, the overlay may get clipped. No flip-above-if-no-room logic yet.
3. **Calendar doesn't highlight "today."** Only the selected date is highlighted. `chrono::Local::now()` could be compared and styled differently.
4. **No password/masked text input.** TextField could gain a `.masked()` option.
5. **Mouse click-to-focus needs app wiring.** `FocusManager::focus_at` exists; the app's run loop must call it when a mouse click arrives (tuile doesn't do this automatically because it doesn't own the run loop).

## How the app at ../ynab-budget-manager uses tuile

`ynab-budget-manager-subproject-1` is a real consumer of tuile. Specifically:
- The Budget tab's category group headers use `Theme::dark()` semantic tokens (`primary`/`on_primary`, `surface_raised`/`on_surface_strong`) for Section 508 contrast.
- The Schedule modal (`src/tui/schedule_modal.rs` in that repo) is a three-mode state machine (ReadOnly / Write / field-editing) built on `TextField`, `DollarField`, `DateField`, `Dropdown`, `Radio`, `Toggle`. It demonstrates the "while typing, shortcuts don't fire" pattern: the modal checks `is_editing_field()` (any widget with `editing == true` or `open == true`) and routes all keys to the focused widget when true.

This integration is what proved out the widget ergonomics — if you're iterating on tuile, running the host app is a fast feedback loop for rendering and focus issues that unit tests can't catch.

## Repo conventions

- **Commits do not include `Co-Authored-By:` trailers.** Just subject + optional body.
- **Edition 2024**, MSRV 1.85.
- **No features flags yet.** Everything is default-on.

## Tests you can run right now

```
cargo test           # 78 tests across widgets, containers, scroll, theme contrast, focus, event conversion
cargo build          # clean, no warnings
```

Theme presets are asserted at AA contrast:
```
cargo test theme::tests
```
- `dark_theme_passes_aa` — checks `on_surface/surface`, `on_primary/primary`, `on_error/error`, `on_surface_strong/surface` all ≥ 4.5:1
- `light_theme_passes_aa` — same, against the light preset

## Where to start reading the code

1. `src/lib.rs` — 5 seconds, see what's exported.
2. `src/component.rs` — the core trait.
3. `src/event.rs` and `src/action.rs` — the input/output vocabulary.
4. `src/theme/tokens.rs` + `src/theme/mod.rs` — the palette.
5. `src/widgets/text_field.rs` — the prototype widget that other field widgets compose.
6. `src/widgets/toggle.rs` — small, clean example with the explicit `letter_t_does_not_flip` test.
7. `src/containers/vstack.rs` — small, clean container example.
