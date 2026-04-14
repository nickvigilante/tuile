//! The `Component` trait and its rendering context.

use crate::action::Action;
use crate::event::Event;
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Context passed to `handle_event`. Contains cross-cutting state like the
/// current theme and (eventually) the focus manager.
pub struct Context<'a> {
    pub theme: &'a Theme,
}

/// Context passed to `render`. Contains the current effective theme (with any
/// ancestor overrides applied).
pub struct RenderContext<'a> {
    pub theme: &'a Theme,
}

impl<'a> RenderContext<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    /// Render a child with a theme override. The override function mutates a
    /// clone of the current theme; the child sees the modified theme.
    pub fn with_theme<F, R>(&self, modify: impl FnOnce(&mut Theme), render: F) -> R
    where
        F: FnOnce(&RenderContext) -> R,
    {
        let mut overridden = self.theme.clone();
        modify(&mut overridden);
        let child_ctx = RenderContext::new(&overridden);
        render(&child_ctx)
    }
}

/// All UI components implement this trait.
pub trait Component {
    /// Handle an input event. Return an `Action` describing what happened.
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> Action;

    /// Render the component into the given area.
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderContext);

    /// Whether this component can receive focus. Default: true.
    /// Override with `false` for display-only widgets like labels or status text.
    fn is_focusable(&self) -> bool {
        true
    }

    /// Whether this component traps focus (Tab won't escape it).
    /// Default: false. Modals override to `true`.
    fn traps_focus(&self) -> bool {
        false
    }

    /// Children this component owns, if any. Default: empty.
    /// Layout containers (VStack, Grid, Form) override this. Leaf widgets
    /// (TextField, Toggle) don't.
    fn children_mut<'a>(&'a mut self) -> Vec<&'a mut (dyn Component + 'a)> {
        Vec::new()
    }

    /// Read-only view of children (for focus walking without mutation).
    fn children(&self) -> Vec<&dyn Component> {
        Vec::new()
    }

    /// Debug identifier (optional, helps with focus-stack inspection).
    fn name(&self) -> &'static str {
        "Component"
    }
}
