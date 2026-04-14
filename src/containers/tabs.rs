//! Tabs container: a tab bar on top, a single active panel below.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Tabs as RatatuiTabs};
use ratatui::Frame;

pub struct Tabs {
    titles: Vec<String>,
    panels: Vec<Box<dyn Component>>,
    pub active: usize,
}

impl Tabs {
    pub fn new() -> Self { Self { titles: Vec::new(), panels: Vec::new(), active: 0 } }
    pub fn add(mut self, title: impl Into<String>, panel: Box<dyn Component>) -> Self {
        self.titles.push(title.into()); self.panels.push(panel); self
    }
    pub fn active_panel_mut<'a>(&'a mut self) -> Option<&'a mut (dyn Component + 'a)> {
        self.panels.get_mut(self.active).map(|b| -> &'a mut (dyn Component + 'a) { b.as_mut() })
    }
}

impl Default for Tabs {
    fn default() -> Self { Self::new() }
}

impl Component for Tabs {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        if let Event::Key(k) = event {
            match k.code {
                KeyCode::Tab => {
                    self.active = (self.active + 1) % self.titles.len().max(1);
                    return Action::Absorbed;
                }
                KeyCode::BackTab => {
                    self.active = if self.active == 0 { self.titles.len().saturating_sub(1) } else { self.active - 1 };
                    return Action::Absorbed;
                }
                _ => {}
            }
        }
        if let Some(panel) = self.panels.get_mut(self.active) {
            panel.handle_event(event, ctx)
        } else {
            Action::Ignored
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        let titles: Vec<Line> = self.titles.iter().map(|t| Line::from(t.clone())).collect();
        let tabs = RatatuiTabs::new(titles)
            .block(Block::default().borders(Borders::ALL))
            .select(self.active)
            .highlight_style(Style::default().fg(ctx.theme.primary).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, chunks[0]);
        if let Some(panel) = self.panels.get(self.active) {
            panel.render(frame, chunks[1], ctx);
        }
    }

    fn name(&self) -> &'static str { "Tabs" }
}
