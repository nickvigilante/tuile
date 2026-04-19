//! Horizontal radio group for selecting one of ≤5 options.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use crate::theme::Theme;
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
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

impl Radio {
    fn build_paragraph<'a>(&'a self, theme: &Theme) -> Paragraph<'a> {
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
        Paragraph::new(Line::from(spans))
    }
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
        frame.render_widget(self.build_paragraph(ctx.theme), area);
    }

    fn name(&self) -> &'static str { "Radio" }

    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

impl ScrollContent for Radio {
    fn measure(&self, _width: u16) -> u16 { 1 }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        self.build_paragraph(ctx.theme).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_content::ScrollContent;
    use ratatui::buffer::Buffer;

    #[test]
    fn scroll_content_measure_is_one() {
        let r = Radio::new("x", vec!["a".into(), "b".into()], 0);
        assert_eq!(r.measure(20), 1);
    }

    #[test]
    fn scroll_content_render_buf_writes_label_and_options() {
        let r = Radio::new("x", vec!["a".into(), "b".into()], 0);
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        r.render_buf(&mut buf, area, &rctx);
        let row: String = (0..area.width).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(row.contains("a"), "row was {:?}", row);
        assert!(row.contains("b"), "row was {:?}", row);
    }

    #[test]
    fn as_scroll_content_returns_self() {
        let r = Radio::new("x", vec!["a".into()], 0);
        assert!(r.as_scroll_content().is_some());
    }
}
