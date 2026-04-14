//! Horizontal stack container: renders children left-to-right with optional spacing.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct HStack {
    children: Vec<Box<dyn Component>>,
    spacing: u16,
}

impl HStack {
    pub fn new() -> Self { Self { children: Vec::new(), spacing: 0 } }
    pub fn spacing(mut self, n: u16) -> Self { self.spacing = n; self }
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
}
