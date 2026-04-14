//! Scroll container: wraps a child component, handles keyboard/wheel scroll
//! events. True virtual-buffer clipping is a TODO for v0.2; for v0.1, data
//! widgets (List, Table) manage their own scroll and this container mainly
//! routes Page/Home/End/wheel events.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::{Event, MouseKind};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct ScrollView {
    child: Box<dyn Component>,
    pub scroll: u16,
    pub content_height: u16,
}

impl ScrollView {
    pub fn new(child: Box<dyn Component>) -> Self {
        Self { child, scroll: 0, content_height: 0 }
    }
    pub fn set_content_height(&mut self, h: u16) {
        self.content_height = h;
        let max = self.content_height.saturating_sub(self.scroll);
        if self.scroll > max { self.scroll = max; }
    }
}

impl Component for ScrollView {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        match event {
            Event::Key(k) => match k.code {
                KeyCode::PageDown => { self.scroll = self.scroll.saturating_add(10); Action::Absorbed }
                KeyCode::PageUp => { self.scroll = self.scroll.saturating_sub(10); Action::Absorbed }
                KeyCode::Home => { self.scroll = 0; Action::Absorbed }
                KeyCode::End => { self.scroll = self.content_height; Action::Absorbed }
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
        self.child.render(frame, area, ctx);
    }

    fn name(&self) -> &'static str { "ScrollView" }
}
