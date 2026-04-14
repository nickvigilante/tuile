//! Single-line text input with cursor, dirty tracking, and validation.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::validation::ValidationResult;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct TextField {
    value: String,
    cursor: usize,
    committed: String,
    pub editing: bool,
    required: bool,
    pub label: String,
    char_filter: Option<fn(char) -> bool>,
}

impl TextField {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        let v: String = value.into();
        Self {
            cursor: v.len(),
            committed: v.clone(),
            value: v,
            editing: false,
            required: false,
            label: label.into(),
            char_filter: None,
        }
    }

    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn char_filter(mut self, f: fn(char) -> bool) -> Self { self.char_filter = Some(f); self }

    pub fn value(&self) -> &str { &self.value }
    pub fn set_value(&mut self, v: impl Into<String>) {
        self.value = v.into();
        self.cursor = self.value.len();
        self.committed = self.value.clone();
    }
    pub fn is_dirty(&self) -> bool { self.value != self.committed }
    pub fn commit(&mut self) { self.committed = self.value.clone(); }
    pub fn revert(&mut self) {
        self.value = self.committed.clone();
        self.cursor = self.value.len();
    }
    pub fn start_editing(&mut self) { self.editing = true; self.cursor = self.value.len(); }
    pub fn stop_editing(&mut self) { self.editing = false; }

    pub fn validate(&self) -> ValidationResult {
        if self.required && self.value.trim().is_empty() {
            ValidationResult::Invalid("Required".to_string())
        } else {
            ValidationResult::Valid
        }
    }
}

impl Component for TextField {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        let Event::Key(k) = event else { return Action::Ignored; };

        // Not editing: only Enter begins editing.
        if !self.editing {
            if k.code == KeyCode::Enter {
                self.start_editing();
                return Action::Absorbed;
            }
            return Action::Ignored;
        }

        // Editing mode:
        match k.code {
            KeyCode::Char(ch) => {
                if let Some(filter) = self.char_filter {
                    if !filter(ch) { return Action::Absorbed; }
                }
                self.value.insert(self.cursor, ch);
                self.cursor += ch.len_utf8();
                Action::Changed
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let prev = self.value[..self.cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    self.value.replace_range(prev..self.cursor, "");
                    self.cursor = prev;
                    Action::Changed
                } else { Action::Absorbed }
            }
            KeyCode::Delete => {
                if self.cursor < self.value.len() {
                    let next = self.value[self.cursor..]
                        .char_indices().nth(1).map(|(i, _)| self.cursor + i)
                        .unwrap_or(self.value.len());
                    self.value.replace_range(self.cursor..next, "");
                    Action::Changed
                } else { Action::Absorbed }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor = self.value[..self.cursor]
                        .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                }
                Action::Absorbed
            }
            KeyCode::Right => {
                if self.cursor < self.value.len() {
                    self.cursor = self.value[self.cursor..]
                        .char_indices().nth(1).map(|(i, _)| self.cursor + i)
                        .unwrap_or(self.value.len());
                }
                Action::Absorbed
            }
            KeyCode::Home => { self.cursor = 0; Action::Absorbed }
            KeyCode::End => { self.cursor = self.value.len(); Action::Absorbed }
            KeyCode::Enter => { self.editing = false; self.commit(); Action::Submit }
            KeyCode::Esc => { self.editing = false; self.revert(); Action::Cancel }
            _ => Action::Absorbed,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
        let is_invalid = matches!(self.validate(), ValidationResult::Invalid(_));

        let label_style = theme.label_style(self.editing);
        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };

        let value_style = if is_invalid {
            Style::default().fg(theme.error)
        } else if self.editing {
            theme.focused_style()
        } else {
            theme.unfocused_style()
        };

        let spans = if self.editing {
            let before = &self.value[..self.cursor];
            let after = &self.value[self.cursor..];
            vec![
                Span::styled(format!("{}: ", self.label), label_style),
                Span::styled("[", label_style),
                Span::styled(format!(" {}", before), value_style),
                Span::styled("▏", Style::default().fg(theme.cursor).add_modifier(Modifier::SLOW_BLINK)),
                Span::styled(format!("{} ", after), value_style),
                Span::styled("]", label_style),
                dirty,
            ]
        } else {
            let display = if self.value.is_empty() { "(none)".to_string() } else { self.value.clone() };
            vec![
                Span::styled(format!("{}: ", self.label), label_style),
                Span::styled(format!(" {} ", display), value_style),
                dirty,
            ]
        };
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn name(&self) -> &'static str { "TextField" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(c: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code: c,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn typing_when_not_editing_is_ignored() {
        let mut f = TextField::new("Name", "");
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        assert!(matches!(f.handle_event(&key(KeyCode::Char('a')), &mut c), Action::Ignored));
    }

    #[test]
    fn enter_begins_editing() {
        let mut f = TextField::new("Name", "");
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        f.handle_event(&key(KeyCode::Enter), &mut c);
        assert!(f.editing);
    }

    #[test]
    fn typing_while_editing_appends() {
        let mut f = TextField::new("Name", "");
        f.start_editing();
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        f.handle_event(&key(KeyCode::Char('a')), &mut c);
        f.handle_event(&key(KeyCode::Char('b')), &mut c);
        assert_eq!(f.value(), "ab");
    }

    #[test]
    fn enter_commits_and_stops_editing() {
        let mut f = TextField::new("Name", "old");
        f.start_editing();
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        f.handle_event(&key(KeyCode::Char('!')), &mut c);
        assert!(f.is_dirty());
        let a = f.handle_event(&key(KeyCode::Enter), &mut c);
        assert!(matches!(a, Action::Submit));
        assert!(!f.editing);
        assert!(!f.is_dirty());
    }

    #[test]
    fn esc_reverts() {
        let mut f = TextField::new("Name", "old");
        f.start_editing();
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        f.handle_event(&key(KeyCode::Char('!')), &mut c);
        f.handle_event(&key(KeyCode::Esc), &mut c);
        assert_eq!(f.value(), "old");
    }

    #[test]
    fn required_validates() {
        let f = TextField::new("Name", "").required();
        assert!(matches!(f.validate(), ValidationResult::Invalid(_)));
    }

    #[test]
    fn char_filter_rejects() {
        let mut f = TextField::new("Digits", "").char_filter(|c| c.is_ascii_digit());
        f.start_editing();
        let theme = Theme::dark();
        let mut c = Context { theme: &theme };
        f.handle_event(&key(KeyCode::Char('x')), &mut c);
        assert_eq!(f.value(), "");
        f.handle_event(&key(KeyCode::Char('5')), &mut c);
        assert_eq!(f.value(), "5");
    }
}
