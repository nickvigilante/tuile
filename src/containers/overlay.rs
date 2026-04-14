//! Overlay container: renders a child on top of the current area at a
//! caller-specified Rect. Used internally by Dropdown and (future) ContextMenu.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use ratatui::Frame;

pub struct Overlay {
    child: Box<dyn Component>,
    pub area: Rect,
    pub visible: bool,
}

impl Overlay {
    pub fn new(child: Box<dyn Component>) -> Self {
        Self { child, area: Rect::default(), visible: false }
    }
    pub fn show(&mut self, area: Rect) { self.area = area; self.visible = true; }
    pub fn hide(&mut self) { self.visible = false; }
    pub fn child_mut(&mut self) -> &mut dyn Component { self.child.as_mut() }
}

impl Component for Overlay {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        if self.visible { self.child.handle_event(event, ctx) } else { Action::Ignored }
    }

    fn render(&self, frame: &mut Frame, _area: Rect, ctx: &RenderContext) {
        if self.visible {
            frame.render_widget(Clear, self.area);
            self.child.render(frame, self.area, ctx);
        }
    }

    fn traps_focus(&self) -> bool { self.visible }
    fn name(&self) -> &'static str { "Overlay" }
}
