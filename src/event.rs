//! Unified input event type.

use crossterm::event::{
    KeyEvent as CtKeyEvent, MouseEvent as CtMouseEvent, MouseEventKind as CtMouseKind,
    MouseButton as CtMouseButton,
};

/// All input events a component might receive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Keyboard key press.
    Key(CtKeyEvent),
    /// Bracketed paste. Already sanitized (no CR, no LF).
    Paste(String),
    /// Mouse event with absolute terminal coordinates.
    Mouse(MouseEvent),
    /// Terminal resize. Width, height.
    Resize(u16, u16),
    /// A tick from the render clock (for spinners, blinking cursors, etc.).
    Tick,
}

/// Mouse event with semantic kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: crossterm::event::KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl Event {
    /// Convert a crossterm Event into a tuile Event.
    /// Returns `None` for events we don't model (focus, kitty, etc.).
    pub fn from_crossterm(ev: crossterm::event::Event) -> Option<Self> {
        use crossterm::event::Event as CtEvent;
        match ev {
            CtEvent::Key(k) => Some(Event::Key(k)),
            CtEvent::Paste(s) => {
                let sanitized: String = s.chars().filter(|&c| c != '\r' && c != '\n').collect();
                Some(Event::Paste(sanitized))
            }
            CtEvent::Mouse(m) => Some(Event::Mouse(convert_mouse(m))),
            CtEvent::Resize(w, h) => Some(Event::Resize(w, h)),
            _ => None,
        }
    }
}

fn convert_mouse(m: CtMouseEvent) -> MouseEvent {
    let kind = match m.kind {
        CtMouseKind::Down(b) => MouseKind::Down(convert_button(b)),
        CtMouseKind::Up(b) => MouseKind::Up(convert_button(b)),
        CtMouseKind::Drag(b) => MouseKind::Drag(convert_button(b)),
        CtMouseKind::Moved => MouseKind::Moved,
        CtMouseKind::ScrollUp => MouseKind::ScrollUp,
        CtMouseKind::ScrollDown => MouseKind::ScrollDown,
        CtMouseKind::ScrollLeft => MouseKind::ScrollLeft,
        CtMouseKind::ScrollRight => MouseKind::ScrollRight,
    };
    MouseEvent {
        kind,
        column: m.column,
        row: m.row,
        modifiers: m.modifiers,
    }
}

fn convert_button(b: CtMouseButton) -> MouseButton {
    match b {
        CtMouseButton::Left => MouseButton::Left,
        CtMouseButton::Right => MouseButton::Right,
        CtMouseButton::Middle => MouseButton::Middle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    #[test]
    fn paste_sanitizes_newlines() {
        let ev = Event::from_crossterm(crossterm::event::Event::Paste(
            "hello\nworld\r".to_string(),
        ));
        assert_eq!(ev, Some(Event::Paste("helloworld".to_string())));
    }

    #[test]
    fn key_round_trip() {
        let key = CtKeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let ev = Event::from_crossterm(crossterm::event::Event::Key(key));
        assert!(matches!(ev, Some(Event::Key(_))));
    }
}
