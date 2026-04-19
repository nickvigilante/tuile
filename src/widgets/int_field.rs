//! Integer input with range validation and Up/Down increment/decrement.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::validation::ValidationResult;
use crate::widgets::text_field::TextField;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct IntField {
    inner: TextField,
    min: Option<i64>,
    max: Option<i64>,
    allow_negative: bool,
}

impl IntField {
    pub fn new(label: impl Into<String>, value: i64) -> Self {
        let inner = TextField::new(label, value.to_string())
            .char_filter(|c| c.is_ascii_digit() || c == '-');
        Self { inner, min: None, max: None, allow_negative: true }
    }
    pub fn required(mut self) -> Self { self.inner = self.inner.required(); self }
    pub fn range(mut self, min: i64, max: i64) -> Self {
        self.min = Some(min); self.max = Some(max);
        if min >= 0 { self.allow_negative = false; }
        self
    }
    pub fn value_i64(&self) -> Option<i64> { self.inner.value().parse().ok() }
    pub fn set_value(&mut self, v: i64) { self.inner.set_value(v.to_string()); }
    pub fn is_dirty(&self) -> bool { self.inner.is_dirty() }
    pub fn editing(&self) -> bool { self.inner.editing }
    pub fn start_editing(&mut self) { self.inner.start_editing(); }

    pub fn validate(&self) -> ValidationResult {
        let base = self.inner.validate();
        if !matches!(base, ValidationResult::Valid) { return base; }
        if self.inner.value().trim().is_empty() { return ValidationResult::Valid; }
        match self.inner.value().parse::<i64>() {
            Err(_) => ValidationResult::Invalid("Must be a whole number".into()),
            Ok(n) => {
                if let Some(min) = self.min {
                    if n < min { return ValidationResult::Invalid(format!("Min {}", min)); }
                }
                if let Some(max) = self.max {
                    if n > max { return ValidationResult::Invalid(format!("Max {}", max)); }
                }
                ValidationResult::Valid
            }
        }
    }
}

impl Component for IntField {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        if self.inner.editing {
            if let Event::Key(k) = event {
                match k.code {
                    KeyCode::Up => {
                        if let Some(n) = self.value_i64() {
                            let next = n.saturating_add(1);
                            if self.max.is_none_or(|m| next <= m) {
                                self.inner.set_value(next.to_string());
                                self.inner.start_editing();
                                return Action::Changed;
                            }
                        }
                        return Action::Absorbed;
                    }
                    KeyCode::Down => {
                        if let Some(n) = self.value_i64() {
                            let next = n.saturating_sub(1);
                            if self.min.is_none_or(|m| next >= m) {
                                self.inner.set_value(next.to_string());
                                self.inner.start_editing();
                                return Action::Changed;
                            }
                        }
                        return Action::Absorbed;
                    }
                    KeyCode::Char('-') if !self.allow_negative => return Action::Absorbed,
                    _ => {}
                }
            }
        }
        self.inner.handle_event(event, ctx)
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        self.inner.render(frame, area, ctx);
    }

    fn name(&self) -> &'static str { "IntField" }
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
    fn up_increments() {
        let mut f = IntField::new("Count", 5);
        f.start_editing();
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        f.handle_event(&key(KeyCode::Up), &mut c);
        assert_eq!(f.value_i64(), Some(6));
    }

    #[test]
    fn down_decrements() {
        let mut f = IntField::new("Count", 5);
        f.start_editing();
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        f.handle_event(&key(KeyCode::Down), &mut c);
        assert_eq!(f.value_i64(), Some(4));
    }

    #[test]
    fn range_clamps() {
        let mut f = IntField::new("Count", 10).range(0, 10);
        f.start_editing();
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        f.handle_event(&key(KeyCode::Up), &mut c);
        assert_eq!(f.value_i64(), Some(10));
    }
}
