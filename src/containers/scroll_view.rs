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
        let mag = delta.unsigned_abs().min(u16::MAX as u32) as u16;
        self.scroll = if delta >= 0 {
            self.scroll.saturating_add(mag)
        } else {
            self.scroll.saturating_sub(mag)
        };
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
