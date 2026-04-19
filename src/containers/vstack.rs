//! Vertical stack container: renders children top-to-bottom with optional spacing.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct VStack {
    children: Vec<Box<dyn Component>>,
    spacing: u16,
}

impl VStack {
    pub fn new() -> Self { Self { children: Vec::new(), spacing: 0 } }
    pub fn spacing(mut self, n: u16) -> Self { self.spacing = n; self }
    #[allow(clippy::should_implement_trait)] // deliberate builder-API choice; not std::ops::Add
    pub fn add(mut self, child: Box<dyn Component>) -> Self {
        self.children.push(child); self
    }
    pub fn push(&mut self, child: Box<dyn Component>) { self.children.push(child); }
}

impl Default for VStack {
    fn default() -> Self { Self::new() }
}

impl Component for VStack {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        for child in &mut self.children {
            let a = child.handle_event(event, ctx);
            if a.is_handled() {
                return a;
            }
        }
        Action::Ignored
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let n = self.children.len() as u16;
        if n == 0 || area.height == 0 { return; }

        let total_spacing = self.spacing.saturating_mul(n.saturating_sub(1));
        let usable = area.height.saturating_sub(total_spacing);
        let per_child = usable / n.max(1);
        let remainder = usable % n.max(1);

        let mut y = area.y;
        for (i, child) in self.children.iter().enumerate() {
            let extra = if (i as u16) < remainder { 1 } else { 0 };
            let h = per_child + extra;
            if h == 0 { continue; }
            let rect = Rect { x: area.x, y, width: area.width, height: h };
            child.render(frame, rect, ctx);
            y = y.saturating_add(h).saturating_add(self.spacing);
        }
    }

    fn children(&self) -> Vec<&dyn Component> {
        self.children.iter().map(|c| c.as_ref()).collect()
    }

    fn children_mut<'a>(&'a mut self) -> Vec<&'a mut (dyn Component + 'a)> {
        self.children.iter_mut().map(|c| -> &'a mut (dyn Component + 'a) { c.as_mut() }).collect()
    }

    fn is_focusable(&self) -> bool { false }

    fn name(&self) -> &'static str { "VStack" }

    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use crate::widgets::text::Text;
    use ratatui::layout::Rect;

    struct Leaf;
    impl Component for Leaf {
        fn handle_event(&mut self, _: &Event, _: &mut Context) -> Action { Action::Absorbed }
        fn render(&self, _: &mut Frame, _: Rect, _: &RenderContext) {}
    }

    #[test]
    fn children_count_matches() {
        let stack = VStack::new().add(Box::new(Leaf)).add(Box::new(Leaf));
        assert_eq!(stack.children().len(), 2);
    }

    #[test]
    fn is_not_focusable() {
        let stack = VStack::new();
        assert!(!stack.is_focusable());
    }

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

    #[test]
    fn as_scroll_content_returns_self() {
        let stack = VStack::new();
        assert!(stack.as_scroll_content().is_some());
    }
}
