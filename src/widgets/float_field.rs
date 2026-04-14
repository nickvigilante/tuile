//! Floating-point input (primarily for percentages) with range validation.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::validation::ValidationResult;
use crate::widgets::text_field::TextField;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct FloatField {
    inner: TextField,
    min: Option<f64>,
    max: Option<f64>,
    decimals: usize,
}

impl FloatField {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        let inner = TextField::new(label, format!("{}", value))
            .char_filter(|c| c.is_ascii_digit() || c == '.' || c == '-');
        Self { inner, min: None, max: None, decimals: 2 }
    }
    pub fn required(mut self) -> Self { self.inner = self.inner.required(); self }
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min); self.max = Some(max); self
    }
    pub fn decimals(mut self, n: usize) -> Self { self.decimals = n; self }
    pub fn value_f64(&self) -> Option<f64> { self.inner.value().parse().ok() }
    pub fn set_value(&mut self, v: f64) {
        self.inner.set_value(format!("{:.*}", self.decimals, v));
    }
    pub fn is_dirty(&self) -> bool { self.inner.is_dirty() }
    pub fn editing(&self) -> bool { self.inner.editing }
    pub fn start_editing(&mut self) { self.inner.start_editing(); }

    pub fn validate(&self) -> ValidationResult {
        let base = self.inner.validate();
        if !matches!(base, ValidationResult::Valid) { return base; }
        if self.inner.value().trim().is_empty() { return ValidationResult::Valid; }
        match self.inner.value().parse::<f64>() {
            Err(_) => ValidationResult::Invalid("Must be a number".into()),
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

impl Component for FloatField {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        self.inner.handle_event(event, ctx)
    }
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        self.inner.render(frame, area, ctx);
    }
    fn name(&self) -> &'static str { "FloatField" }
}
