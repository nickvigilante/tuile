//! Form container: vertical field list with Up/Down navigation between fields.
//! Tracks which child is currently "focused" internally.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct Form {
    fields: Vec<Box<dyn Component>>,
    pub focus_idx: usize,
    pub field_editing: bool,
}

impl Form {
    pub fn new() -> Self { Self { fields: Vec::new(), focus_idx: 0, field_editing: false } }
    #[allow(clippy::should_implement_trait)] // deliberate builder-API choice; not std::ops::Add
    pub fn add(mut self, field: Box<dyn Component>) -> Self { self.fields.push(field); self }
    pub fn push(&mut self, field: Box<dyn Component>) { self.fields.push(field); }
    pub fn focused_field_mut<'a>(&'a mut self) -> Option<&'a mut (dyn Component + 'a)> {
        self.fields.get_mut(self.focus_idx).map(|b| -> &'a mut (dyn Component + 'a) { b.as_mut() })
    }
}

impl Default for Form {
    fn default() -> Self { Self::new() }
}

impl Component for Form {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        if self.field_editing {
            if let Some(f) = self.fields.get_mut(self.focus_idx) {
                let a = f.handle_event(event, ctx);
                if matches!(a, Action::Submit | Action::Cancel) {
                    self.field_editing = false;
                }
                return a;
            }
        }
        if let Event::Key(k) = event {
            match k.code {
                KeyCode::Up => {
                    if self.focus_idx > 0 { self.focus_idx -= 1; }
                    return Action::Absorbed;
                }
                KeyCode::Down => {
                    if self.focus_idx + 1 < self.fields.len() { self.focus_idx += 1; }
                    return Action::Absorbed;
                }
                KeyCode::Enter => {
                    self.field_editing = true;
                    if let Some(f) = self.fields.get_mut(self.focus_idx) {
                        return f.handle_event(event, ctx);
                    }
                    return Action::Absorbed;
                }
                _ => {}
            }
        }
        Action::Ignored
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        let n = self.fields.len() as u16;
        if n == 0 || area.height == 0 { return; }
        let per = area.height / n.max(1);
        let rem = area.height % n.max(1);
        let mut y = area.y;
        for (i, f) in self.fields.iter().enumerate() {
            let extra = if (i as u16) < rem { 1 } else { 0 };
            let h = per + extra;
            let rect = Rect { x: area.x, y, width: area.width, height: h };
            f.render(frame, rect, ctx);
            y += h;
        }
    }

    fn name(&self) -> &'static str { "Form" }
}
