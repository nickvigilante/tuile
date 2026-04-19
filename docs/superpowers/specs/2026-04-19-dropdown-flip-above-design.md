# Design: Dropdown flip-above

Status: proposed (v0.2)
Date: 2026-04-19

## Motivation

`Dropdown`'s overlay (v0.2) always opens below its anchor. When the anchor is near the bottom of the terminal, ratatui's viewport clamping silently reduces the overlay's height — options get hidden with no indicator. Users have no way to see or interact with the clipped options.

This spec defines flip-above behavior: when there is not enough room below, the overlay opens upward instead. This resolves Known v0.2 limitation #2 ("Dropdown overlay only opens below the anchor").

## Scope

In scope:

- Flip the overlay above the anchor when below doesn't fit.
- Extract the placement rule into a pure helper so it is unit-testable without a frame.
- Preserve today's internal overlay layout (filter line at top, options below).
- Preserve today's fallback clamping behavior when neither side has enough room.

Out of scope:

- Horizontal flip when the anchor is near the right edge. Current right-clipping is acceptable for v0.2.
- Animated flip transitions. TUI.
- Changing `max_visible = 8` or the `min_width = 30` constants.

## Design

### Placement rule

1. `desired_h = filtered.len().min(8) + 3` — `+3` for filter line (1) and top+bottom border (2).
2. `room_below = screen.height - (anchor.y - screen.y + 1)` (saturating).
3. `room_above = anchor.y - screen.y` (saturating).
4. If `desired_h <= room_below`: open below, full `desired_h`.
5. Else if `room_above > room_below`: open above, height clamped to `room_above`.
6. Else: open below, height clamped to `room_below` (v0.2 fallback, unchanged).

Width is always `anchor.width.max(30)` regardless of direction.

### New helper

In `src/widgets/dropdown.rs`, add a private free function:

```rust
fn overlay_rect(anchor: Rect, screen: Rect, desired_h: u16) -> Rect {
    let width = anchor.width.max(30);
    let room_below = screen
        .height
        .saturating_sub(anchor.y.saturating_sub(screen.y) + 1);
    let room_above = anchor.y.saturating_sub(screen.y);

    if desired_h <= room_below {
        Rect { x: anchor.x, y: anchor.y + 1, width, height: desired_h }
    } else if room_above > room_below {
        let h = desired_h.min(room_above);
        Rect { x: anchor.x, y: anchor.y.saturating_sub(h), width, height: h }
    } else {
        Rect { x: anchor.x, y: anchor.y + 1, width, height: room_below }
    }
}
```

Notes:

- Pure function — no `&self`, no `Frame`. Takes the three inputs it needs and returns a `Rect`.
- `screen` is passed in (as `frame.area()`) so the math works in a sub-frame / nested-rendering scenario. Today's callers always pass `frame.area()`.
- `saturating_sub`/`saturating_add` throughout — no panic when anchor is at `y == 0` or `y == screen.y + screen.height - 1`.

### `render()` refactor

The existing inline placement block is replaced:

```rust
if self.open {
    let filtered = self.filtered();
    let desired_h = (filtered.len().min(8) + 3) as u16;
    let overlay = overlay_rect(area, frame.area(), desired_h);
    frame.render_widget(Clear, overlay);
    // ... existing filter+options rendering, unchanged ...
}
```

The overlay's internal layout (filter line → options → optional "… N more" footer) does not change. Only the `Rect` is different when flipped.

## Testing

Five new unit tests in the existing `#[cfg(test)] mod tests`:

- `opens_below_when_fits` — desired fits below → opens below, full height.
- `flips_above_when_below_insufficient_and_above_has_more_room` — cramped below with plenty above → flips above.
- `clamps_above_when_room_above_smaller_than_desired` (see note below) — when flipping, height is clamped to `room_above`; when staying below, height is clamped to `room_below`.
- `stays_below_at_top_edge_when_no_room_above` — anchor at y=0 with cramped screen → stays below with clamped height.
- `width_respects_minimum_30` — narrow anchor still gets a 30-wide overlay.

All tests assert on returned `Rect` values — no `TestBackend`, no rendering. Existing behavior tests (`enter_opens`, `filter_reduces`, `enter_selects`, `allow_create_adds`) remain unchanged.

> Note on `clamps_above_when_room_above_smaller_than_desired`: in the actual test, a case where the rule chooses below and clamps is slightly easier to construct than one where the rule chooses above and clamps. The plan will include both flavors so the clamp branch is exercised regardless of which side is chosen.

## SPEC.md updates (follow-up after implementation)

- Remove item 2 from "Known v0.2 limitations" ("Dropdown overlay only opens below...").
- Append one sentence to the Dropdown catalog entry: "Opens below the anchor by default; flips above automatically when below doesn't have enough room."

## Breaking changes

None. No API signature changes, no new public types, no behavioral regressions for anchors where the overlay already fits below. Consumers observe only the improvement.
