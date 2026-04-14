//! Date input as three IntFields: YYYY/MM/DD. Left/Right moves between sub-fields,
//! Up/Down on the focused sub-field increments/decrements via IntField.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::validation::ValidationResult;
use crate::widgets::int_field::IntField;
use chrono::{Datelike, NaiveDate};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sub { Year, Month, Day }

pub struct DateField {
    year: IntField,
    month: IntField,
    day: IntField,
    focus: Sub,
    pub label: String,
    editing: bool,
    committed: Option<NaiveDate>,
}

impl DateField {
    pub fn new(label: impl Into<String>, date: Option<NaiveDate>) -> Self {
        let (y, m, d) = match date {
            Some(dt) => (dt.year() as i64, dt.month() as i64, dt.day() as i64),
            None => (2026, 1, 1),
        };
        Self {
            year: IntField::new("Y", y).range(1900, 2999),
            month: IntField::new("M", m).range(1, 12),
            day: IntField::new("D", d).range(1, 31),
            focus: Sub::Year,
            label: label.into(),
            editing: false,
            committed: date,
        }
    }

    pub fn value(&self) -> Option<NaiveDate> {
        let y = self.year.value_i64()?;
        let m = self.month.value_i64()?;
        let d = self.day.value_i64()?;
        NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32)
    }

    pub fn set_value(&mut self, date: Option<NaiveDate>) {
        match date {
            Some(d) => {
                self.year.set_value(d.year() as i64);
                self.month.set_value(d.month() as i64);
                self.day.set_value(d.day() as i64);
            }
            None => {
                self.year.set_value(2026);
                self.month.set_value(1);
                self.day.set_value(1);
            }
        }
        self.committed = date;
    }

    pub fn is_dirty(&self) -> bool { self.value() != self.committed }
    pub fn editing(&self) -> bool { self.editing }
    pub fn start_editing(&mut self) {
        self.editing = true;
        self.focus = Sub::Year;
        self.year.start_editing();
    }

    pub fn validate(&self) -> ValidationResult {
        if self.value().is_some() { ValidationResult::Valid }
        else { ValidationResult::Invalid("Invalid date".into()) }
    }
}

impl Component for DateField {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        if !self.editing {
            if let Event::Key(k) = event {
                if k.code == KeyCode::Enter {
                    self.start_editing();
                    return Action::Absorbed;
                }
            }
            return Action::Ignored;
        }
        if let Event::Key(k) = event {
            match k.code {
                KeyCode::Left => {
                    self.focus = match self.focus {
                        Sub::Day => Sub::Month,
                        Sub::Month => Sub::Year,
                        _ => self.focus,
                    };
                    return Action::Absorbed;
                }
                KeyCode::Right => {
                    self.focus = match self.focus {
                        Sub::Year => Sub::Month,
                        Sub::Month => Sub::Day,
                        _ => self.focus,
                    };
                    return Action::Absorbed;
                }
                KeyCode::Esc => {
                    self.set_value(self.committed);
                    self.editing = false;
                    return Action::Cancel;
                }
                KeyCode::Enter => {
                    if let Some(d) = self.value() {
                        self.committed = Some(d);
                        self.editing = false;
                        return Action::Submit;
                    }
                    return Action::Absorbed;
                }
                _ => {}
            }
        }
        match self.focus {
            Sub::Year => self.year.handle_event(event, ctx),
            Sub::Month => self.month.handle_event(event, ctx),
            Sub::Day => self.day.handle_event(event, ctx),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
        let label_style = theme.label_style(self.editing);

        let fmt_part = |val: Option<i64>, digits: usize, focused: bool| -> Span<'static> {
            let s = match val {
                Some(v) => format!("{:0width$}", v, width = digits),
                None => "?".repeat(digits),
            };
            if focused && self.editing {
                Span::styled(s, theme.focused_style())
            } else {
                Span::styled(s, theme.unfocused_style())
            }
        };

        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };

        let spans = vec![
            Span::styled(format!("{}: ", self.label), label_style),
            fmt_part(self.year.value_i64(), 4, self.focus == Sub::Year),
            Span::styled("/", label_style),
            fmt_part(self.month.value_i64(), 2, self.focus == Sub::Month),
            Span::styled("/", label_style),
            fmt_part(self.day.value_i64(), 2, self.focus == Sub::Day),
            dirty,
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn name(&self) -> &'static str { "DateField" }
}
