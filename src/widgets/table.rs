//! Scrollable table with columns. Each row is Vec<String> matching columns.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::{Event, MouseKind};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table as RatatuiTable};
use ratatui::Frame;

pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    pub cursor: usize,
    pub scroll: u16,
    widths: Vec<Constraint>,
}

impl Table {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let widths = headers.iter().map(|_| Constraint::Fill(1)).collect();
        Self { headers, rows, cursor: 0, scroll: 0, widths }
    }
    pub fn widths(mut self, w: Vec<Constraint>) -> Self { self.widths = w; self }
    pub fn selected_row(&self) -> Option<&Vec<String>> { self.rows.get(self.cursor) }
    pub fn selected_index(&self) -> usize { self.cursor }
    pub fn row_count(&self) -> usize { self.rows.len() }
    pub fn set_rows(&mut self, rows: Vec<Vec<String>>) {
        self.rows = rows;
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
    }
}

impl Component for Table {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        match event {
            Event::Key(k) => match k.code {
                KeyCode::Up => { if self.cursor > 0 { self.cursor -= 1; } Action::Changed }
                KeyCode::Down => {
                    if self.cursor + 1 < self.rows.len() { self.cursor += 1; }
                    Action::Changed
                }
                KeyCode::PageUp => { self.cursor = self.cursor.saturating_sub(10); Action::Changed }
                KeyCode::PageDown => {
                    self.cursor = (self.cursor + 10).min(self.rows.len().saturating_sub(1));
                    Action::Changed
                }
                KeyCode::Home => { self.cursor = 0; Action::Changed }
                KeyCode::End => { self.cursor = self.rows.len().saturating_sub(1); Action::Changed }
                KeyCode::Enter => Action::Submit,
                _ => Action::Ignored,
            },
            Event::Mouse(m) => match m.kind {
                MouseKind::ScrollDown => {
                    self.cursor = (self.cursor + 3).min(self.rows.len().saturating_sub(1));
                    Action::Changed
                }
                MouseKind::ScrollUp => { self.cursor = self.cursor.saturating_sub(3); Action::Changed }
                _ => Action::Ignored,
            },
            _ => Action::Ignored,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
        let header = Row::new(self.headers.iter().map(|h| Cell::from(h.as_str())).collect::<Vec<_>>())
            .style(Style::default().fg(theme.on_surface_strong).add_modifier(Modifier::BOLD));

        let body_h = area.height.saturating_sub(2) as usize;
        let mut scroll = self.scroll as usize;
        if self.cursor < scroll { scroll = self.cursor; }
        else if self.cursor >= scroll + body_h.max(1) {
            scroll = self.cursor + 1 - body_h.max(1);
        }
        if !self.rows.is_empty() && body_h > 0 && scroll + body_h > self.rows.len() {
            scroll = self.rows.len().saturating_sub(body_h);
        }

        let end = (scroll + body_h).min(self.rows.len());
        let rows: Vec<Row> = self.rows[scroll..end].iter().enumerate().map(|(i, r)| {
            let idx = scroll + i;
            let style = if idx == self.cursor {
                Style::default().fg(theme.on_primary).bg(theme.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.on_surface)
            };
            Row::new(r.iter().map(|c| Cell::from(c.as_str())).collect::<Vec<_>>()).style(style)
        }).collect();

        let tbl = RatatuiTable::new(rows, self.widths.clone()).header(header);
        frame.render_widget(tbl, area);
    }

    fn name(&self) -> &'static str { "Table" }
}
