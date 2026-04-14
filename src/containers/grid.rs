//! Grid container: fixed rows × columns. Child at (r, c) occupies cell (r, c).

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use ratatui::layout::Rect;
use ratatui::Frame;

pub struct Grid {
    rows: u16,
    cols: u16,
    cells: Vec<Vec<Option<Box<dyn Component>>>>,
}

impl Grid {
    pub fn new(rows: u16, cols: u16) -> Self {
        let cells = (0..rows).map(|_| (0..cols).map(|_| None).collect()).collect();
        Self { rows, cols, cells }
    }

    pub fn set(mut self, row: u16, col: u16, child: Box<dyn Component>) -> Self {
        if let Some(r) = self.cells.get_mut(row as usize) {
            if let Some(c) = r.get_mut(col as usize) {
                *c = Some(child);
            }
        }
        self
    }
}

impl Component for Grid {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action {
        for row in &mut self.cells {
            for cell in row.iter_mut().flatten() {
                let a = cell.handle_event(event, ctx);
                if a.is_handled() { return a; }
            }
        }
        Action::Ignored
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        if self.rows == 0 || self.cols == 0 { return; }
        let row_h = area.height / self.rows;
        let col_w = area.width / self.cols;
        for (r, row) in self.cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if let Some(comp) = cell {
                    let rect = Rect {
                        x: area.x + (c as u16) * col_w,
                        y: area.y + (r as u16) * row_h,
                        width: col_w,
                        height: row_h,
                    };
                    comp.render(frame, rect, ctx);
                }
            }
        }
    }

    fn is_focusable(&self) -> bool { false }
    fn name(&self) -> &'static str { "Grid" }
}
