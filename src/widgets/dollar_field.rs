//! Dollar amount input. Stores milliunits internally (1000 per dollar, matching YNAB).

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::validation::ValidationResult;
use crate::widgets::text_field::TextField;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct DollarField {
    inner: TextField,
}

impl DollarField {
    pub fn new(label: impl Into<String>, milliunits: i64) -> Self {
        let inner = TextField::new(label, milliunits_to_display(milliunits))
            .char_filter(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '$' || c == '-');
        Self { inner }
    }
    pub fn required(mut self) -> Self { self.inner = self.inner.required(); self }
    pub fn value_milliunits(&self) -> Option<i64> { parse_dollar_to_milliunits(self.inner.value()) }
    pub fn set_milliunits(&mut self, m: i64) { self.inner.set_value(milliunits_to_display(m)); }
    pub fn is_dirty(&self) -> bool { self.inner.is_dirty() }
    pub fn editing(&self) -> bool { self.inner.editing }
    pub fn start_editing(&mut self) { self.inner.start_editing(); }

    pub fn validate(&self) -> ValidationResult {
        let base = self.inner.validate();
        if !matches!(base, ValidationResult::Valid) { return base; }
        if self.inner.value().trim().is_empty() { return ValidationResult::Valid; }
        match parse_dollar_to_milliunits(self.inner.value()) {
            Some(_) => ValidationResult::Valid,
            None => ValidationResult::Invalid("Invalid dollar amount (e.g. 150.00)".into()),
        }
    }
}

impl Component for DollarField {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        self.inner.handle_event(event, ctx)
    }
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        self.inner.render(frame, area, ctx);
    }
    fn name(&self) -> &'static str { "DollarField" }
}

fn milliunits_to_display(m: i64) -> String {
    let sign = if m < 0 { "-" } else { "" };
    let abs = m.unsigned_abs();
    format!("{}{}.{:02}", sign, abs / 1000, (abs % 1000) / 10)
}

pub fn parse_dollar_to_milliunits(input: &str) -> Option<i64> {
    let s = input.trim().replace([',', '$'], "");
    if s.is_empty() { return None; }
    if let Some(dot) = s.find('.') {
        if s.len() - dot - 1 > 2 { return None; }
    }
    let val: f64 = s.parse().ok()?;
    Some((val * 1000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer() { assert_eq!(parse_dollar_to_milliunits("150"), Some(150_000)); }
    #[test]
    fn parse_decimal() { assert_eq!(parse_dollar_to_milliunits("150.50"), Some(150_500)); }
    #[test]
    fn parse_dollar_sign() { assert_eq!(parse_dollar_to_milliunits("$150.00"), Some(150_000)); }
    #[test]
    fn parse_comma() { assert_eq!(parse_dollar_to_milliunits("1,234.56"), Some(1_234_560)); }
    #[test]
    fn rejects_too_precise() { assert_eq!(parse_dollar_to_milliunits("1.123"), None); }
    #[test]
    fn round_trip() {
        assert_eq!(milliunits_to_display(150_500), "150.50");
    }
}
