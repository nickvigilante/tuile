//! Dropdown widget with fuzzy filtering and optional create-new.
//! When closed, shows selected value. Enter opens; typing filters; Enter selects.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::validation::ValidationResult;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub struct Dropdown {
    options: Vec<String>,
    selected: Option<usize>,
    committed: Option<usize>,
    pub label: String,
    pub open: bool,
    filter: String,
    cursor: usize,
    required: bool,
    allow_create: bool,
}

impl Dropdown {
    pub fn new(label: impl Into<String>, options: Vec<String>, selected: Option<usize>) -> Self {
        Self {
            options,
            selected,
            committed: selected,
            label: label.into(),
            open: false,
            filter: String::new(),
            cursor: 0,
            required: false,
            allow_create: false,
        }
    }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn allow_create(mut self) -> Self { self.allow_create = true; self }
    pub fn selected_value(&self) -> Option<&str> {
        self.selected.and_then(|i| self.options.get(i).map(|s| s.as_str()))
    }
    pub fn selected_index(&self) -> Option<usize> { self.selected }
    pub fn set_options(&mut self, opts: Vec<String>) {
        self.options = opts;
        if let Some(i) = self.selected {
            if i >= self.options.len() { self.selected = None; }
        }
        self.committed = self.selected;
    }
    pub fn set_selected_by_value(&mut self, v: &str) {
        self.selected = self.options.iter().position(|o| o == v);
        self.committed = self.selected;
    }
    pub fn is_dirty(&self) -> bool { self.selected != self.committed }
    pub fn commit(&mut self) { self.committed = self.selected; }
    pub fn revert(&mut self) { self.selected = self.committed; }
    pub fn validate(&self) -> ValidationResult {
        if self.required && self.selected.is_none() {
            ValidationResult::Invalid("Selection required".into())
        } else { ValidationResult::Valid }
    }

    fn filtered(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            (0..self.options.len()).collect()
        } else {
            let l = self.filter.to_ascii_lowercase();
            self.options.iter().enumerate()
                .filter(|(_, o)| o.to_ascii_lowercase().contains(&l))
                .map(|(i, _)| i).collect()
        }
    }
}

impl Component for Dropdown {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        let Event::Key(k) = event else { return Action::Ignored; };
        if !self.open {
            match k.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.open = true;
                    self.filter.clear();
                    self.cursor = 0;
                    Action::Absorbed
                }
                KeyCode::Esc => { self.revert(); Action::Cancel }
                _ => Action::Ignored,
            }
        } else {
            let filtered = self.filtered();
            match k.code {
                KeyCode::Esc => { self.open = false; self.filter.clear(); Action::Cancel }
                KeyCode::Up => { if self.cursor > 0 { self.cursor -= 1; } Action::Absorbed }
                KeyCode::Down => {
                    if !filtered.is_empty() && self.cursor + 1 < filtered.len() {
                        self.cursor += 1;
                    }
                    Action::Absorbed
                }
                KeyCode::Enter => {
                    if let Some(&orig) = filtered.get(self.cursor) {
                        self.selected = Some(orig);
                        self.commit();
                        self.open = false;
                        self.filter.clear();
                        Action::Submit
                    } else if self.allow_create && !self.filter.is_empty() {
                        self.options.push(self.filter.clone());
                        self.selected = Some(self.options.len() - 1);
                        self.commit();
                        self.open = false;
                        self.filter.clear();
                        Action::Submit
                    } else { Action::Absorbed }
                }
                KeyCode::Char(ch) => { self.filter.push(ch); self.cursor = 0; Action::Absorbed }
                KeyCode::Backspace => { self.filter.pop(); self.cursor = 0; Action::Absorbed }
                _ => Action::Absorbed,
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let theme = ctx.theme;
        let dirty = if self.is_dirty() {
            Span::styled(" •", Style::default().fg(theme.warning))
        } else { Span::raw("") };

        let display = self.selected_value().unwrap_or("(none)").to_string();
        let arrow = if self.open { "▲" } else { "▼" };

        let line = Line::from(vec![
            Span::styled(format!("{}: ", self.label), theme.label_style(false)),
            Span::styled(display, theme.unfocused_style()),
            Span::styled(format!(" {}", arrow), theme.label_style(self.open)),
            dirty,
        ]);
        frame.render_widget(Paragraph::new(line), area);

        if self.open {
            let filtered = self.filtered();
            let max_visible = 8usize;
            let h = (filtered.len().min(max_visible) + 3) as u16;
            let overlay = Rect {
                x: area.x,
                y: area.y.saturating_add(1),
                width: area.width.max(30),
                height: h.min(frame.area().height.saturating_sub(area.y + 1)),
            };
            frame.render_widget(Clear, overlay);
            let mut lines = vec![
                Line::styled(
                    if self.filter.is_empty() { "Type to filter…".to_string() } else { self.filter.clone() },
                    Style::default().fg(theme.info),
                )
            ];
            for (vis, &orig) in filtered.iter().enumerate().take(max_visible) {
                let opt = &self.options[orig];
                let is_cursor = vis == self.cursor;
                let is_current = self.selected == Some(orig);
                let style = if is_cursor {
                    Style::default().fg(theme.on_primary).bg(theme.primary).add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(theme.primary)
                } else {
                    Style::default().fg(theme.on_surface)
                };
                let prefix = if is_cursor { "› " } else { "  " };
                lines.push(Line::styled(format!("{}{}", prefix, opt), style));
            }
            if filtered.len() > max_visible {
                lines.push(Line::styled(
                    format!("  … {} more", filtered.len() - max_visible),
                    Style::default().fg(theme.on_surface_dim),
                ));
            }
            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default().borders(Borders::ALL).title(format!("{} ▼", self.label))
                        .border_style(Style::default().fg(theme.border_focused))
                ),
                overlay,
            );
        }
    }

    fn name(&self) -> &'static str { "Dropdown" }
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

    fn opts() -> Vec<String> { vec!["Apple".into(), "Banana".into(), "Cherry".into()] }

    #[test]
    fn enter_opens() {
        let mut d = Dropdown::new("x", opts(), None);
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        d.handle_event(&key(KeyCode::Enter), &mut c);
        assert!(d.open);
    }

    #[test]
    fn filter_reduces() {
        let mut d = Dropdown::new("x", opts(), None);
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        d.handle_event(&key(KeyCode::Enter), &mut c);
        d.handle_event(&key(KeyCode::Char('a')), &mut c);
        assert_eq!(d.filtered().len(), 2); // Apple, Banana (both contain 'a')
    }

    #[test]
    fn enter_selects() {
        let mut d = Dropdown::new("x", opts(), None);
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        d.handle_event(&key(KeyCode::Enter), &mut c);
        d.handle_event(&key(KeyCode::Down), &mut c);
        d.handle_event(&key(KeyCode::Enter), &mut c);
        assert_eq!(d.selected_value(), Some("Banana"));
    }

    #[test]
    fn allow_create_adds() {
        let mut d = Dropdown::new("x", opts(), None).allow_create();
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        d.handle_event(&key(KeyCode::Enter), &mut c);
        for ch in "Kiwi".chars() {
            d.handle_event(&key(KeyCode::Char(ch)), &mut c);
        }
        d.handle_event(&key(KeyCode::Enter), &mut c);
        assert_eq!(d.selected_value(), Some("Kiwi"));
    }
}
