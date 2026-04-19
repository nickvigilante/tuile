//! Focus management: a stack of focus scopes, each with its own cursor.

use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(u64);

#[derive(Debug, Default)]
struct FocusScope {
    focusables: Vec<FocusEntry>,
    /// Index into `focusables` of the current focus.
    current: Option<usize>,
    /// ID to restore after the next register call if the index isn't known yet.
    pending_restore: Option<FocusId>,
}

#[derive(Debug, Clone)]
struct FocusEntry {
    id: FocusId,
    rect: Rect,
}

#[derive(Debug)]
pub struct FocusManager {
    scopes: Vec<FocusScope>,
    next_id: u64,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            scopes: vec![FocusScope::default()],
            next_id: 0,
        }
    }

    pub fn new_id(&mut self) -> FocusId {
        let id = FocusId(self.next_id);
        self.next_id += 1;
        id
    }

    fn scope(&self) -> &FocusScope {
        self.scopes.last().expect("focus stack never empty")
    }
    fn scope_mut(&mut self) -> &mut FocusScope {
        self.scopes.last_mut().expect("focus stack never empty")
    }

    /// Call at the start of each render pass. Clears registered rects but
    /// remembers the previously focused id for restoration.
    pub fn begin_frame(&mut self) {
        let scope = self.scope_mut();
        let current_id = scope.current.and_then(|i| scope.focusables.get(i).map(|e| e.id));
        scope.focusables.clear();
        scope.current = None;
        scope.pending_restore = current_id;
    }

    /// Register a focusable component rendered this frame.
    pub fn register(&mut self, id: FocusId, rect: Rect) {
        let scope = self.scope_mut();
        scope.focusables.push(FocusEntry { id, rect });
        let last_idx = scope.focusables.len() - 1;
        if scope.pending_restore == Some(id) {
            scope.current = Some(last_idx);
            scope.pending_restore = None;
        } else if scope.current.is_none() && scope.pending_restore.is_none() {
            scope.current = Some(last_idx);
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(FocusScope::default());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn focused(&self) -> Option<FocusId> {
        let s = self.scope();
        s.current.and_then(|i| s.focusables.get(i).map(|e| e.id))
    }

    pub fn focus_next(&mut self) {
        let s = self.scope_mut();
        if s.focusables.is_empty() { return; }
        s.current = Some(match s.current {
            Some(i) => (i + 1) % s.focusables.len(),
            None => 0,
        });
    }

    pub fn focus_prev(&mut self) {
        let s = self.scope_mut();
        if s.focusables.is_empty() { return; }
        s.current = Some(match s.current {
            Some(0) => s.focusables.len() - 1,
            Some(i) => i - 1,
            None => 0,
        });
    }

    pub fn focus_at(&mut self, column: u16, row: u16) -> bool {
        let s = self.scope_mut();
        for (i, entry) in s.focusables.iter().enumerate() {
            let r = entry.rect;
            if column >= r.x
                && column < r.x.saturating_add(r.width)
                && row >= r.y
                && row < r.y.saturating_add(r.height)
            {
                s.current = Some(i);
                return true;
            }
        }
        false
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, width: w, height: h }
    }

    #[test]
    fn focus_next_cycles() {
        let mut fm = FocusManager::new();
        let a = fm.new_id();
        let b = fm.new_id();
        let c = fm.new_id();
        fm.begin_frame();
        fm.register(a, rect(0, 0, 10, 1));
        fm.register(b, rect(0, 1, 10, 1));
        fm.register(c, rect(0, 2, 10, 1));
        assert_eq!(fm.focused(), Some(a));
        fm.focus_next();
        assert_eq!(fm.focused(), Some(b));
        fm.focus_next();
        assert_eq!(fm.focused(), Some(c));
        fm.focus_next();
        assert_eq!(fm.focused(), Some(a)); // wraps
    }

    #[test]
    fn focus_prev_wraps() {
        let mut fm = FocusManager::new();
        let a = fm.new_id();
        let b = fm.new_id();
        fm.begin_frame();
        fm.register(a, rect(0, 0, 10, 1));
        fm.register(b, rect(0, 1, 10, 1));
        fm.focus_prev();
        assert_eq!(fm.focused(), Some(b));
    }

    #[test]
    fn focus_at_hits_rect() {
        let mut fm = FocusManager::new();
        let a = fm.new_id();
        let b = fm.new_id();
        fm.begin_frame();
        fm.register(a, rect(0, 0, 10, 1));
        fm.register(b, rect(0, 1, 10, 1));
        assert!(fm.focus_at(5, 1));
        assert_eq!(fm.focused(), Some(b));
    }

    #[test]
    fn push_pop_scope_isolates_focus() {
        let mut fm = FocusManager::new();
        let a = fm.new_id();
        fm.begin_frame();
        fm.register(a, rect(0, 0, 10, 1));
        fm.push_scope();
        let b = fm.new_id();
        fm.register(b, rect(0, 2, 10, 1));
        assert_eq!(fm.focused(), Some(b));
        fm.pop_scope();
        assert_eq!(fm.focused(), Some(a));
    }

    #[test]
    fn focus_preserved_across_frames() {
        let mut fm = FocusManager::new();
        let a = fm.new_id();
        let b = fm.new_id();
        fm.begin_frame();
        fm.register(a, rect(0, 0, 10, 1));
        fm.register(b, rect(0, 1, 10, 1));
        fm.focus_next();
        assert_eq!(fm.focused(), Some(b));
        // Next frame re-registers
        fm.begin_frame();
        fm.register(a, rect(0, 0, 10, 1));
        fm.register(b, rect(0, 1, 10, 1));
        assert_eq!(fm.focused(), Some(b)); // preserved
    }
}
