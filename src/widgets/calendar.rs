//! Month calendar widget. Arrow keys to navigate days, PgUp/PgDn for months.
//! Enter submits the selected date. Esc reverts.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use chrono::{Datelike, Duration, NaiveDate};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct Calendar {
    pub selected: NaiveDate,
    committed: NaiveDate,
}

impl Calendar {
    pub fn new(initial: NaiveDate) -> Self { Self { selected: initial, committed: initial } }
    pub fn value(&self) -> NaiveDate { self.selected }
    pub fn set_value(&mut self, d: NaiveDate) { self.selected = d; self.committed = d; }
    pub fn is_dirty(&self) -> bool { self.selected != self.committed }
}

impl Component for Calendar {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        let Event::Key(k) = event else { return Action::Ignored; };
        match k.code {
            KeyCode::Left => {
                self.selected = self.selected.pred_opt().unwrap_or(self.selected);
                Action::Changed
            }
            KeyCode::Right => {
                self.selected = self.selected.succ_opt().unwrap_or(self.selected);
                Action::Changed
            }
            KeyCode::Up => {
                self.selected = self.selected - Duration::days(7);
                Action::Changed
            }
            KeyCode::Down => {
                self.selected = self.selected + Duration::days(7);
                Action::Changed
            }
            KeyCode::PageUp => {
                let m = self.selected.month();
                let new = if m == 1 {
                    self.selected.with_year(self.selected.year() - 1).and_then(|d| d.with_month(12))
                } else {
                    self.selected.with_month(m - 1)
                };
                self.selected = new.unwrap_or(self.selected);
                Action::Changed
            }
            KeyCode::PageDown => {
                let m = self.selected.month();
                let new = if m == 12 {
                    self.selected.with_year(self.selected.year() + 1).and_then(|d| d.with_month(1))
                } else {
                    self.selected.with_month(m + 1)
                };
                self.selected = new.unwrap_or(self.selected);
                Action::Changed
            }
            KeyCode::Enter => { self.committed = self.selected; Action::Submit }
            KeyCode::Esc => { self.selected = self.committed; Action::Cancel }
            _ => Action::Absorbed,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
        let first = NaiveDate::from_ymd_opt(self.selected.year(), self.selected.month(), 1).unwrap();
        let days_in_month = {
            let next_month = if first.month() == 12 {
                NaiveDate::from_ymd_opt(first.year() + 1, 1, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1).unwrap()
            };
            (next_month - Duration::days(1)).day()
        };
        let title = format!(" {} {} ", month_name(first.month()), first.year());
        let mut lines = vec![Line::styled(
            "Mo Tu We Th Fr Sa Su".to_string(),
            Style::default().fg(theme.on_surface_strong).add_modifier(Modifier::BOLD),
        )];

        let weekday_mon_zero = first.weekday().num_days_from_monday();
        let mut row: Vec<Span> = Vec::new();
        for _ in 0..weekday_mon_zero { row.push(Span::raw("   ")); }
        for d in 1..=days_in_month {
            let day_date = NaiveDate::from_ymd_opt(first.year(), first.month(), d).unwrap();
            let style = if day_date == self.selected {
                Style::default().fg(theme.on_primary).bg(theme.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.on_surface)
            };
            row.push(Span::styled(format!("{:>2} ", d), style));
            if row.len() == 7 {
                lines.push(Line::from(std::mem::take(&mut row)));
            }
        }
        if !row.is_empty() { lines.push(Line::from(row)); }

        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default().title(title).borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_focused))
            ),
            area,
        );
    }

    fn name(&self) -> &'static str { "Calendar" }
}

fn month_name(m: u32) -> &'static str {
    ["", "January", "February", "March", "April", "May", "June",
     "July", "August", "September", "October", "November", "December"][m as usize]
}
