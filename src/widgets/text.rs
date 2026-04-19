//! Read-only text block. Wraps `ratatui::widgets::Paragraph`. Usable as
//! scrollable content inside `ScrollView`.

use crate::action::Action;
use crate::component::{Component, Context, RenderContext};
use crate::event::Event;
use crate::scroll_content::ScrollContent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui::Frame;

pub struct Text {
    content: String,
    wrap: bool,
    alignment: Alignment,
}

impl Text {
    pub fn new(s: impl Into<String>) -> Self {
        Self { content: s.into(), wrap: true, alignment: Alignment::Left }
    }
    pub fn no_wrap(mut self) -> Self { self.wrap = false; self }
    pub fn alignment(mut self, a: Alignment) -> Self { self.alignment = a; self }

    fn build_paragraph(&self) -> Paragraph<'_> {
        let mut p = Paragraph::new(self.content.as_str()).alignment(self.alignment);
        if self.wrap { p = p.wrap(Wrap { trim: false }); }
        p
    }
}

impl Component for Text {
    fn handle_event(&mut self, _: &Event, _: &mut Context) -> Action { Action::Ignored }
    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &RenderContext) {
        frame.render_widget(self.build_paragraph(), area);
    }
    fn is_focusable(&self) -> bool { false }
    fn name(&self) -> &'static str { "Text" }
    fn as_scroll_content(&self) -> Option<&dyn ScrollContent> { Some(self) }
}

impl ScrollContent for Text {
    fn measure(&self, width: u16) -> u16 {
        if width == 0 { return 0; }
        let w = width as usize;
        let mut total: u32 = 0;
        for line in self.content.split('\n') {
            total = total.saturating_add(count_wrapped_lines(line, w, self.wrap));
        }
        total.min(u16::MAX as u32) as u16
    }
    fn render_buf(&self, buf: &mut Buffer, area: Rect, _ctx: &RenderContext) {
        self.build_paragraph().render(area, buf);
    }
}

fn count_wrapped_lines(line: &str, width: usize, wrap: bool) -> u32 {
    if !wrap {
        return 1;
    }
    if line.chars().all(char::is_whitespace) {
        return 1;
    }
    let mut visual_lines: u32 = 1;
    let mut pos: usize = 0;
    for word in line.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > width {
            // Word is wider than the viewport. Break it across lines.
            // First fragment ends the current line (if pos > 0, emit a new line
            // so this long word starts fresh); the word then takes
            // ceil(word_len / width) lines on its own, with the remainder
            // becoming the new pos.
            if pos > 0 {
                visual_lines += 1;
            }
            let extra_full_lines = (word_len / width) as u32;
            visual_lines = visual_lines.saturating_add(extra_full_lines.saturating_sub(1));
            pos = word_len % width;
            if pos == 0 && word_len > 0 {
                // Exact fit: cursor is at end-of-line. Reset to 0 so the next
                // word starts on a fresh line.
                visual_lines += 1;
                pos = 0;
            }
            continue;
        }
        let needed = if pos == 0 { word_len } else { pos + 1 + word_len };
        if needed <= width {
            pos = needed;
        } else {
            visual_lines += 1;
            pos = word_len;
        }
    }
    visual_lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rctx<'a>(theme: &'a crate::theme::Theme) -> RenderContext<'a> {
        RenderContext::new(theme)
    }

    #[test]
    fn measure_single_line_unwrapped() {
        let t = Text::new("hello world");
        assert_eq!(t.measure(20), 1);
    }

    #[test]
    fn measure_wraps_at_width() {
        let t = Text::new("hello world");
        // "hello " + "world" wraps at width 6 into 2 lines. At width 5,
        // "hello" + "world" also → 2 lines (word-boundary wrap).
        assert_eq!(t.measure(5), 2);
    }

    #[test]
    fn measure_multiline_literal() {
        let t = Text::new("a\nb\nc");
        assert_eq!(t.measure(20), 3);
    }

    #[test]
    fn measure_width_zero_returns_zero() {
        let t = Text::new("hi");
        assert_eq!(t.measure(0), 0);
    }

    #[test]
    fn render_buf_writes_content() {
        let t = Text::new("hi");
        let theme = crate::theme::Theme::dark();
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        t.render_buf(&mut buf, area, &rctx(&theme));
        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(1, 0)].symbol(), "i");
    }

    #[test]
    fn is_not_focusable() {
        let t = Text::new("x");
        assert!(!t.is_focusable());
    }
}
