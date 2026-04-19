//! Grid container: fixed rows × columns. Child at (r, c) occupies cell (r, c).

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
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
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

impl ScrollContent for Grid {
    fn measure(&self, width: u16) -> u16 {
        if self.rows == 0 || self.cols == 0 { return 0; }
        let col_w = width / self.cols;
        let mut total: u16 = 0;
        for row in &self.cells {
            let row_max: u16 = row
                .iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(|c| c.as_scroll_content())
                        .map(|sc| sc.measure(col_w))
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0);
            total = total.saturating_add(row_max);
        }
        total
    }

    fn render_buf(&self, buf: &mut Buffer, area: Rect, ctx: &RenderContext) {
        if self.rows == 0 || self.cols == 0 { return; }
        let col_w = area.width / self.cols;
        let mut y = area.y;
        for row in &self.cells {
            let row_h: u16 = row
                .iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(|c| c.as_scroll_content())
                        .map(|sc| sc.measure(col_w))
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0);
            for (c, cell) in row.iter().enumerate() {
                if let Some(comp) = cell {
                    if let Some(sc) = comp.as_scroll_content() {
                        let rect = Rect {
                            x: area.x + (c as u16) * col_w,
                            y,
                            width: col_w,
                            height: row_h,
                        };
                        sc.render_buf(buf, rect, ctx);
                    }
                }
            }
            y = y.saturating_add(row_h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::text::Text;

    #[test]
    fn measure_sums_row_heights() {
        // 2x2 grid; each cell a Text of height 1.
        let g = Grid::new(2, 2)
            .set(0, 0, Box::new(Text::new("a")))
            .set(0, 1, Box::new(Text::new("b")))
            .set(1, 0, Box::new(Text::new("c")))
            .set(1, 1, Box::new(Text::new("d")));
        assert_eq!(g.measure(10), 2);
    }

    #[test]
    fn measure_takes_max_in_row() {
        // Row 0: cell (0,0) height 1, cell (0,1) height 2 → row 0 = 2.
        // Row 1: one cell of height 1 → row 1 = 1.
        // Total = 3.
        let g = Grid::new(2, 2)
            .set(0, 0, Box::new(Text::new("a")))
            .set(0, 1, Box::new(Text::new("b\nb")))
            .set(1, 0, Box::new(Text::new("c")));
        assert_eq!(g.measure(20), 3);
    }

    #[test]
    fn measure_empty_is_zero() {
        let g = Grid::new(0, 0);
        assert_eq!(g.measure(10), 0);
    }

    #[test]
    fn render_buf_places_cells() {
        let g = Grid::new(1, 2)
            .set(0, 0, Box::new(Text::new("a")))
            .set(0, 1, Box::new(Text::new("b")));
        let theme = crate::theme::Theme::dark();
        let rctx = RenderContext::new(&theme);
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        g.render_buf(&mut buf, area, &rctx);
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(1, 0)].symbol(), "b");
    }

    #[test]
    fn as_scroll_content_returns_self() {
        assert!(Grid::new(1, 1).as_scroll_content().is_some());
    }
}
