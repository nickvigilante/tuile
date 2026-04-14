//! Single-line status bar with auto-dismiss timer (ticks).

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct StatusBar {
    text: String,
    /// Tick countdown. 0 = hidden.
    ticks: u8,
}

impl StatusBar {
    pub fn new() -> Self { Self { text: String::new(), ticks: 0 } }
    pub fn set(&mut self, text: impl Into<String>) { self.text = text.into(); self.ticks = 45; }
    pub fn clear(&mut self) { self.text.clear(); self.ticks = 0; }
    pub fn visible(&self) -> bool { self.ticks > 0 }
}

impl Default for StatusBar {
    fn default() -> Self { Self::new() }
}

impl Component for StatusBar {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        if matches!(event, Event::Tick) && self.ticks > 0 { self.ticks -= 1; }
        Action::Ignored
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        if !self.visible() { return; }
        frame.render_widget(
            Paragraph::new(Line::raw(self.text.clone())).style(Style::default().fg(ctx.theme.info)),
            area,
        );
    }

    fn is_focusable(&self) -> bool { false }
    fn name(&self) -> &'static str { "StatusBar" }
}
