//! Horizontal stack container: renders children left-to-right with optional spacing.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct HStack {
    children: Vec<Box<dyn Component>>,
    spacing: u16,
}

impl HStack {
    pub fn new() -> Self { Self { children: Vec::new(), spacing: 0 } }
    pub fn spacing(mut self, n: u16) -> Self { self.spacing = n; self }
    #[allow(clippy::should_implement_trait)] // deliberate builder-API choice; not std::ops::Add
    pub fn add(mut self, child: Box<dyn Component>) -> Self {
        self.children.push(child); self
    }
    pub fn push(&mut self, child: Box<dyn Component>) { self.children.push(child); }
}

impl Default for HStack {
    fn default() -> Self { Self::new() }
}

impl Component for HStack {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        for child in &mut self.children {
            let a = child.handle_event(event, ctx);
            if a.is_handled() { return a; }
        }
        Action::Ignored
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
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
            child.render(frame, rect, ctx);
            x = x.saturating_add(w).saturating_add(self.spacing);
        }
    }

    fn children(&self) -> Vec<&dyn Component> {
        self.children.iter().map(|c| c.as_ref()).collect()
    }

    fn children_mut<'a>(&'a mut self) -> Vec<&'a mut (dyn Component + 'a)> {
        self.children.iter_mut().map(|c| -> &'a mut (dyn Component + 'a) { c.as_mut() }).collect()
    }

    fn is_focusable(&self) -> bool { false }
    fn name(&self) -> &'static str { "HStack" }
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

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
        // "aa" measures 1. "bb bb bb bb bb" at width 10 wraps to 2 lines.
        let stack = HStack::new()
            .add(Box::new(Text::new("aa")))
            .add(Box::new(Text::new("bb bb bb bb bb")));
        assert_eq!(stack.measure(20), 2);
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
