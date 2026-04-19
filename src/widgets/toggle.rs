//! Binary on/off toggle. Enter or Space to flip. 't' is NOT a shortcut —
//! typing letters inside a field must never accidentally flip this.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use crate::theme::Theme;
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

pub struct Toggle {
    value: bool,
    committed: bool,
    pub label: String,
}

impl Toggle {
    pub fn new(label: impl Into<String>, value: bool) -> Self {
        Self { value, committed: value, label: label.into() }
    }
    pub fn value(&self) -> bool { self.value }
    pub fn set_value(&mut self, v: bool) { self.value = v; self.committed = v; }
    pub fn is_dirty(&self) -> bool { self.value != self.committed }
    pub fn commit(&mut self) { self.committed = self.value; }
    pub fn revert(&mut self) { self.value = self.committed; }

    fn build_paragraph<'a>(&'a self, theme: &Theme) -> Paragraph<'a> {
        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };
        let (text, style) = if self.value {
            ("  ◉ ON ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
        } else {
            (" ○ OFF ", Style::default().fg(theme.on_surface_dim))
        };
        let line = Line::from(vec![
            Span::styled(format!("{}: ", self.label), theme.label_style(false)),
            Span::styled(text, style),
            dirty,
        ]);
        Paragraph::new(line)
    }
}

impl Component for Toggle {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        let Event::Key(k) = event else { return Action::Ignored; };
        match k.code {
            KeyCode::Enter | KeyCode::Char(' ') => { self.value = !self.value; Action::Changed }
            KeyCode::Esc => { self.revert(); Action::Cancel }
            _ => Action::Absorbed,
        }
    }

    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        frame.render_widget(self.build_paragraph(ctx.theme), area);
    }

    fn name(&self) -> &'static str { "Toggle" }
}

impl ScrollContent for Toggle {
    fn measure(&self, _width: u16) -> u16 { 1 }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        self.build_paragraph(ctx.theme).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::scroll_content::ScrollContent;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn key(c: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code: c, modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press, state: KeyEventState::NONE,
        })
    }

    #[test]
    fn enter_flips() {
        let mut t = Toggle::new("x", false);
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        t.handle_event(&key(KeyCode::Enter), &mut c);
        assert!(t.value());
    }

    #[test]
    fn space_flips() {
        let mut t = Toggle::new("x", true);
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        t.handle_event(&key(KeyCode::Char(' ')), &mut c);
        assert!(!t.value());
    }

    #[test]
    fn letter_t_does_not_flip() {
        let mut t = Toggle::new("x", false);
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        t.handle_event(&key(KeyCode::Char('t')), &mut c);
        assert!(!t.value(), "'t' must not be a toggle shortcut");
    }

    #[test]
    fn scroll_content_measure_is_one() {
        let t = Toggle::new("x", false);
        assert_eq!(t.measure(20), 1);
    }

    #[test]
    fn scroll_content_render_buf_writes_on_label() {
        let t = Toggle::new("x", true);
        let theme = Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        t.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(row.starts_with("x:"), "row was {:?}", row);
        assert!(row.contains("◉"), "row was {:?}", row);
        assert!(row.contains("ON"), "row was {:?}", row);
    }

    #[test]
    fn as_scroll_content_returns_self() {
        let t = Toggle::new("x", false);
        assert!(t.as_scroll_content().is_some());
    }
}
