//! Modal container: centered overlay with a titled, bordered frame that
//! traps focus while open.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear};
use ratatui::Frame;

pub struct Modal {
    child: Box<dyn Component>,
    title: String,
    width_pct: u16,
    height_pct: u16,
    pub open: bool,
}

impl Modal {
    pub fn new(title: impl Into<String>, child: Box<dyn Component>) -> Self {
        Self { child, title: title.into(), width_pct: 70, height_pct: 70, open: false }
    }
    pub fn size_pct(mut self, width: u16, height: u16) -> Self {
        self.width_pct = width; self.height_pct = height; self
    }
    pub fn show(&mut self) { self.open = true; }
    pub fn hide(&mut self) { self.open = false; }
    pub fn child_mut(&mut self) -> &mut dyn Component { self.child.as_mut() }
}

fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vert[1])[1]
}

impl Component for Modal {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        if !self.open { return Action::Ignored; }
        self.child.handle_event(event, ctx)
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        if !self.open { return; }
        let rect = centered_rect(self.width_pct, self.height_pct, area);
        frame.render_widget(Clear, rect);
        let block = Block::default()
            .title(self.title.clone())
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(ctx.theme.border_focused));
        let inner = block.inner(rect);
        frame.render_widget(block, rect);
        self.child.render(frame, inner, ctx);
    }

    fn traps_focus(&self) -> bool { self.open }
    fn name(&self) -> &'static str { "Modal" }
}
