//! Binary on/off toggle. Enter or Space to flip. 't' is NOT a shortcut —
//! typing letters inside a field must never accidentally flip this.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
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

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
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
        frame.render_widget(Paragraph::new(line), area);
    }

    fn name(&self) -> &'static str { "Toggle" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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
}
