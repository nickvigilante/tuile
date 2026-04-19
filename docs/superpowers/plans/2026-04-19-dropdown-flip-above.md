# Dropdown Flip-Above Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Dropdown` open upward when there isn't enough room below the anchor, using a pure placement helper that's unit-testable without a frame.

**Architecture:** Extract a private free function `overlay_rect(anchor, screen, desired_h) -> Rect` in `src/widgets/dropdown.rs` that implements the four-branch placement rule (fits-below / flip-above-full / flip-above-clamped / stay-below-clamped). `render()` calls the helper and passes its result to the existing overlay-rendering code — no change to the overlay's internal layout. Five pure unit tests cover all branches.

**Tech Stack:** Rust 2024, ratatui 0.29. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-04-19-dropdown-flip-above-design.md`

---

## Task 1: Add `overlay_rect` helper with unit tests (TDD)

**Files:**
- Modify: `src/widgets/dropdown.rs`

- [ ] **Step 1: Write the failing tests**

Add these imports near the other test-module imports at the top of the existing `#[cfg(test)] mod tests` in `src/widgets/dropdown.rs` (the test module already has `use super::*;`, so `Rect` and `overlay_rect` will resolve once added):

Nothing to import yet — `super::*` covers it. Append these five tests to the bottom of the existing test module:

```rust
    fn screen(w: u16, h: u16) -> Rect { Rect::new(0, 0, w, h) }
    fn anchor_at(y: u16, w: u16) -> Rect { Rect::new(0, y, w.max(30), 1) }

    #[test]
    fn overlay_rect_opens_below_when_fits() {
        // Anchor near top of a 40-row screen, desired 11. Room below = 40 - (5+1) = 34. Fits.
        let r = overlay_rect(anchor_at(5, 30), screen(80, 40), 11);
        assert_eq!(r.y, 6);
        assert_eq!(r.height, 11);
        assert_eq!(r.x, 0);
        assert_eq!(r.width, 30);
    }

    #[test]
    fn overlay_rect_flips_above_when_below_insufficient_and_above_has_more_room() {
        // Anchor at y=35 in 40-row screen. Room below = 40-36 = 4. Room above = 35. Flip.
        let r = overlay_rect(anchor_at(35, 30), screen(80, 40), 11);
        assert_eq!(r.y, 35 - 11);
        assert_eq!(r.height, 11);
    }

    #[test]
    fn overlay_rect_clamps_above_when_room_above_smaller_than_desired() {
        // Anchor at y=8 in 10-row screen, desired 30. Room below = 10-9 = 1. Room above = 8.
        // Above wins (8 > 1), clamp height to 8.
        let r = overlay_rect(anchor_at(8, 30), screen(80, 10), 30);
        assert_eq!(r.y, 0);
        assert_eq!(r.height, 8);
    }

    #[test]
    fn overlay_rect_stays_below_at_top_edge_when_no_room_above() {
        // Anchor at y=0 in 10-row screen, desired 11. Room below = 10-1 = 9. Room above = 0.
        // Below is chosen (9 >= 0), clamp to 9.
        let r = overlay_rect(anchor_at(0, 30), screen(80, 10), 11);
        assert_eq!(r.y, 1);
        assert_eq!(r.height, 9);
    }

    #[test]
    fn overlay_rect_width_respects_minimum_30() {
        // Narrow anchor (width 10) still yields >= 30-wide overlay.
        let r = overlay_rect(Rect::new(0, 5, 10, 1), screen(80, 40), 11);
        assert_eq!(r.width, 30);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib widgets::dropdown::tests`
Expected: compile error — `overlay_rect` not defined.

- [ ] **Step 3: Add the `overlay_rect` function**

In `src/widgets/dropdown.rs`, add this private free function at the bottom of the file (after the `impl Component for Dropdown` block, before the `#[cfg(test)] mod tests` block):

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

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib widgets::dropdown::tests`
Expected: 9 tests pass (4 existing + 5 new).

Then run the full lib suite:

Run: `cargo test --lib`
Expected: 83 tests pass (78 pre-existing + 5 new).

- [ ] **Step 5: Commit**

```bash
git add src/widgets/dropdown.rs
git commit -m "feat(dropdown): add overlay_rect placement helper with tests"
```

---

## Task 2: Wire `overlay_rect` into `render()`

**Files:**
- Modify: `src/widgets/dropdown.rs`

- [ ] **Step 1: Verify no regression test exists yet for rendering position**

There are no rendering-level tests for overlay placement. The existing behavior tests (`enter_opens`, `filter_reduces`, `enter_selects`, `allow_create_adds`) do not touch rendering, so this refactor is covered entirely by the `overlay_rect_*` unit tests from Task 1.

- [ ] **Step 2: Replace the inline overlay-rect calculation in `render()`**

In `src/widgets/dropdown.rs`, inside the `fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext)` method, locate the `if self.open { ... }` block. Currently it begins:

```rust
        if self.open {
            let filtered = self.filtered();
            let max_visible = 8usize;
            let h = (filtered.len().min(max_visible) + 3) as u16;
            let overlay = Rect {
                x: area.x,
                y: area.y.saturating_add(1),
                width: area.width.max(30),
                height: h.min(frame.area().height.saturating_sub(area.y + 1)),
            };
```

Replace those six lines (the `max_visible`, `h`, and `overlay` declarations) with:

```rust
        if self.open {
            let filtered = self.filtered();
            let max_visible = 8usize;
            let desired_h = (filtered.len().min(max_visible) + 3) as u16;
            let overlay = overlay_rect(area, frame.area(), desired_h);
```

The rest of the `if self.open { ... }` body (Clear, Paragraph, Block) stays unchanged. `max_visible` is still used later for the "… N more" footer, so keep it declared.

- [ ] **Step 3: Run tests to verify they still pass**

Run: `cargo test --lib`
Expected: 83 tests pass, 0 failures.

Run: `cargo build 2>&1 | grep -i warning`
Expected: no new warnings on `dropdown.rs`.

- [ ] **Step 4: Commit**

```bash
git add src/widgets/dropdown.rs
git commit -m "feat(dropdown): use overlay_rect in render; flip above when below full"
```

---

## Task 3: Update SPEC.md

**Files:**
- Modify: `SPEC.md`

- [ ] **Step 1: Locate the "Known v0.2 limitations" section**

Run: `grep -n "Known v0.2 limitations" SPEC.md`
Note the starting line number.

- [ ] **Step 2: Remove the Dropdown limitation**

In the "Known v0.2 limitations" numbered list, find and delete this item (it's currently item 2 in that list):

```markdown
2. **Dropdown overlay positions below the anchor.** If the anchor is near the bottom of the screen, the overlay may get clipped. No flip-above-if-no-room logic yet.
```

Renumber any items that came after it (e.g., "3." → "2.", "4." → "3.", etc.).

- [ ] **Step 3: Update the Dropdown catalog entry**

Locate the `### Dropdown` entry in SPEC.md (around the widget catalog section).

Find the bullet list under Dropdown's code example. Append one new bullet at the end of that list:

```markdown
- Overlay opens below the anchor by default; flips above automatically when below doesn't have enough room.
```

- [ ] **Step 4: Verify no stale references remain**

Run: `grep -n "flip-above\|below the anchor" SPEC.md`
Expected: the only match should be the new bullet you just added. No remaining "No flip-above-if-no-room logic yet." language anywhere.

- [ ] **Step 5: Commit**

```bash
git add SPEC.md
git commit -m "docs: Dropdown flip-above is implemented; update SPEC"
```

---

## Task 4: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: 83 lib tests + 1 doc-test (the ScrollContent compile_fail carried over from v0.2), 0 failures.

- [ ] **Step 2: Clean build**

Run: `cargo build 2>&1 | tee /tmp/build.log | tail`
Expected: no warnings.

- [ ] **Step 3: Clippy clean under -D warnings**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean (the pre-existing lint debt was cleared during the v0.2 work).

If any warning appears in `dropdown.rs`, fix it inline before committing. For any warning outside `dropdown.rs`, stop and report it — it indicates a regression from the v0.2 cleanup.

- [ ] **Step 4: Manual visual check (optional)**

If convenient, spin up the `ynab-budget-manager` host app or a small example, and verify that a dropdown whose anchor is near the bottom of the terminal opens upward. Not required for the PR, but useful confirmation.

- [ ] **Step 5: Announce done**

No commit; verification only. If all three verifications pass, the feature is ready for PR.

---

## Plan self-review

**Spec coverage:**
- Placement rule (fits-below / flip-above-full / flip-above-clamped / stay-below-clamped) → Task 1 (helper) + Task 1 tests cover all four branches.
- Extracted pure helper `overlay_rect` → Task 1.
- `render()` wired to use the helper → Task 2.
- Internal overlay layout unchanged → Task 2 explicitly leaves Clear/Paragraph/Block untouched.
- Five unit tests with names matching the spec → Task 1 Step 1.
- SPEC.md updates (remove limitation, add Dropdown bullet) → Task 3.
- Breaking changes: none → no task needed (spec confirms).

**Placeholder scan:** none found. All code blocks contain the actual code to write. All commands are exact.

**Type consistency:** `overlay_rect` signature is identical across Task 1 definition, Task 1 tests (5 call sites), and Task 2 usage in `render()`.

**Test branch coverage check:**
- Branch 1 (`desired_h <= room_below`): `overlay_rect_opens_below_when_fits`.
- Branch 2 (`room_above > room_below`, exact fit): `overlay_rect_flips_above_when_below_insufficient_and_above_has_more_room`.
- Branch 2 clamped (`room_above > room_below`, `desired_h > room_above`): `overlay_rect_clamps_above_when_room_above_smaller_than_desired`.
- Branch 3 (`room_above <= room_below`, clamped below): `overlay_rect_stays_below_at_top_edge_when_no_room_above`.
- Width rule: `overlay_rect_width_respects_minimum_30`.

All four logical branches plus the width rule have explicit tests.
