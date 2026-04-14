//! Scrollable, selectable list. Handles scroll-to-cursor and bottom-clamp.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::{Event, MouseKind};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct List {
    items: Vec<String>,
    pub cursor: usize,
    pub scroll: u16,
}

impl List {
    pub fn new(items: Vec<String>) -> Self { Self { items, cursor: 0, scroll: 0 } }
    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        self.cursor = self.cursor.min(self.items.len().saturating_sub(1));
    }
    pub fn selected(&self) -> Option<&str> { self.items.get(self.cursor).map(|s| s.as_str()) }
    pub fn selected_index(&self) -> usize { self.cursor }

    /// Adjust scroll offset so cursor is in view AND no empty space below content.
    /// Bottom-clamp runs first; cursor-into-view only scrolls forward (down) if
    /// the cursor is past the bottom of the viewport. Scrolling backward is not
    /// done here — the caller should ensure scroll <= cursor when needed.
    /// Public so tests can exercise it directly.
    pub fn adjust_scroll(&mut self, viewport_h: u16) {
        // 1. Bottom-clamp: never show empty space below content.
        let len = self.items.len() as u16;
        if len > 0 && viewport_h > 0 && self.scroll + viewport_h > len {
            self.scroll = len.saturating_sub(viewport_h);
        }
        // 2. Cursor-into-view (only scroll down if cursor is past bottom).
        if (self.cursor as u16) >= self.scroll + viewport_h {
            self.scroll = (self.cursor as u16 + 1).saturating_sub(viewport_h);
        }
    }
}

impl Component for List {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> Action {
        match event {
            Event::Key(k) => match k.code {
                KeyCode::Up => {
                    if self.cursor > 0 { self.cursor -= 1; }
                    Action::Changed
                }
                KeyCode::Down => {
                    if self.cursor + 1 < self.items.len() { self.cursor += 1; }
                    Action::Changed
                }
                KeyCode::Home => { self.cursor = 0; Action::Changed }
                KeyCode::End => { self.cursor = self.items.len().saturating_sub(1); Action::Changed }
                KeyCode::PageDown => {
                    self.cursor = (self.cursor + 10).min(self.items.len().saturating_sub(1));
                    Action::Changed
                }
                KeyCode::PageUp => { self.cursor = self.cursor.saturating_sub(10); Action::Changed }
                KeyCode::Enter => Action::Submit,
                _ => Action::Ignored,
            },
            Event::Mouse(m) => match m.kind {
                MouseKind::ScrollUp => { self.cursor = self.cursor.saturating_sub(3); Action::Changed }
                MouseKind::ScrollDown => {
                    self.cursor = (self.cursor + 3).min(self.items.len().saturating_sub(1));
                    Action::Changed
                }
                _ => Action::Ignored,
            },
            _ => Action::Ignored,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        // Compute local scroll (render can't mutate self).
        let mut scroll = self.scroll;
        if (self.cursor as u16) < scroll { scroll = self.cursor as u16; }
        else if (self.cursor as u16) >= scroll + area.height {
            scroll = (self.cursor as u16 + 1).saturating_sub(area.height);
        }
        let len = self.items.len() as u16;
        if len > 0 && area.height > 0 && scroll + area.height > len {
            scroll = len.saturating_sub(area.height);
        }

        let theme = ctx.theme;
        let start = scroll as usize;
        let end = (start + area.height as usize).min(self.items.len());
        let lines: Vec<Line> = self.items[start..end].iter().enumerate().map(|(i, s)| {
            let idx = start + i;
            if idx == self.cursor {
                Line::styled(
                    format!("› {}", s),
                    Style::default().fg(theme.on_primary).bg(theme.primary).add_modifier(Modifier::BOLD),
                )
            } else {
                Line::styled(format!("  {}", s), Style::default().fg(theme.on_surface))
            }
        }).collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn name(&self) -> &'static str { "List" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    #[test]
    fn cursor_down_moves() {
        let mut l = List::new(vec!["a".into(), "b".into(), "c".into()]);
        let t = Theme::dark();
        let mut c = Context { theme: &t };
        l.handle_event(&Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }), &mut c);
        assert_eq!(l.cursor, 1);
    }

    #[test]
    fn adjust_scroll_bottom_clamps() {
        let mut l = List::new((0..10).map(|i| i.to_string()).collect());
        l.scroll = 8;
        l.adjust_scroll(5);
        // content_len=10, viewport_h=5 → max scroll = 5. So 8 should clamp to 5.
        assert_eq!(l.scroll, 5);
    }
}
