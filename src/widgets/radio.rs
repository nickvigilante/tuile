//! Horizontal radio group for selecting one of ≤5 options.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct Radio {
    options: Vec<String>,
    selected: usize,
    committed: usize,
    pub label: String,
}

impl Radio {
    pub fn new(label: impl Into<String>, options: Vec<String>, selected: usize) -> Self {
        let s = selected.min(options.len().saturating_sub(1));
        Self { options, selected: s, committed: s, label: label.into() }
    }
    pub fn selected_index(&self) -> usize { self.selected }
    pub fn selected_value(&self) -> &str {
        self.options.get(self.selected).map(|s| s.as_str()).unwrap_or("")
    }
    pub fn set_selected(&mut self, idx: usize) {
        self.selected = idx.min(self.options.len().saturating_sub(1));
        self.committed = self.selected;
    }
    pub fn is_dirty(&self) -> bool { self.selected != self.committed }
    pub fn commit(&mut self) { self.committed = self.selected; }
    pub fn revert(&mut self) { self.selected = self.committed; }
}

impl Component for Radio {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        let Event::Key(k) = event else { return Action::Ignored; };
        match k.code {
            KeyCode::Left => {
                if self.selected > 0 { self.selected -= 1; }
                Action::Changed
            }
            KeyCode::Right => {
                if self.selected + 1 < self.options.len() { self.selected += 1; }
                Action::Changed
            }
            KeyCode::Enter => { self.commit(); Action::Submit }
            KeyCode::Esc => { self.revert(); Action::Cancel }
            _ => Action::Absorbed,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };

        let mut spans = vec![Span::styled(format!("{}: ", self.label), theme.label_style(false))];
        for (i, opt) in self.options.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { "◉" } else { "○" };
            let style = if is_selected {
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.on_surface_dim)
            };
            spans.push(Span::styled(format!(" {} {} ", marker, opt), style));
            if i + 1 < self.options.len() {
                spans.push(Span::styled("│", Style::default().fg(theme.divider)));
            }
        }
        spans.push(dirty);
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn name(&self) -> &'static str { "Radio" }
}
