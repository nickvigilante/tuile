//! Vertical stack container: renders children top-to-bottom with optional spacing.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct VStack {
    children: Vec<Box<dyn Component>>,
    spacing: u16,
}

impl VStack {
    pub fn new() -> Self { Self { children: Vec::new(), spacing: 0 } }
    pub fn spacing(mut self, n: u16) -> Self { self.spacing = n; self }
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
