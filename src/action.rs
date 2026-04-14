//! Action returned by a Component's `handle_event`.

use std::any::Any;

/// Result of a component handling an event.
///
/// Most widgets will return `Absorbed` or `Changed`. `Submit`/`Cancel` are
/// used by fields that have an editing mode (e.g., a TextField commits on Enter).
/// `Custom` is an escape hatch for widgets that need to emit a typed value
/// (e.g., a `List` that emits the selected row index).
#[derive(Debug)]
pub enum Action {
    /// Event was consumed; no state change of interest to the parent.
    Absorbed,
    /// The value changed.
    Changed,
    /// The user confirmed a value (Enter on a field, selection in a list).
    Submit,
    /// The user cancelled (Esc on a field).
    Cancel,
    /// An untyped event that bubbles up. Check the component's docs for the
    /// concrete type and downcast.
    Custom(Box<dyn Any + Send>),
    /// Event not handled. Parent may try to handle it.
    Ignored,
}

impl Action {
    /// Returns true if the event was handled (not Ignored).
    pub fn is_handled(&self) -> bool {
        !matches!(self, Action::Ignored)
    }
}
