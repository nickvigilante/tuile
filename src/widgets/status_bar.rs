//! Single-line status bar with auto-dismiss timer (ticks).

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
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
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

impl ScrollContent for StatusBar {
    fn measure(&self, _width: u16) -> u16 { 1 }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        if !self.visible() { return; }
        Paragraph::new(Line::raw(self.text.clone()))
            .style(Style::default().fg(ctx.theme.info))
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn measure_is_one() {
        let b = StatusBar::new();
        assert_eq!(b.measure(20), 1);
    }

    #[test]
    fn render_buf_writes_text_when_visible() {
        let mut b = StatusBar::new();
        b.set("hello");
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        b.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(row.starts_with("hello"), "row was {:?}", row);
    }

    #[test]
    fn render_buf_writes_nothing_when_hidden() {
        let b = StatusBar::new(); // ticks == 0
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        b.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(row.trim(), "");
    }

    #[test]
    fn as_scroll_content_returns_self() {
        let b = StatusBar::new();
        assert!(b.as_scroll_content().is_some());
    }
}
