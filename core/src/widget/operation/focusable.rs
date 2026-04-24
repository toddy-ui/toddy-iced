//! Operate on widgets that can be focused.
use crate::widget::Id;
use crate::widget::operation::accessible::Accessible;
use crate::widget::operation::scrollable::{AbsoluteOffset, RelativeOffset, Scrollable};
use crate::widget::operation::{self, Operation, Outcome};
use crate::{Rectangle, Vector};

/// The internal state of a widget that can be focused.
pub trait Focusable {
    /// Returns whether the widget is focused or not.
    fn is_focused(&self) -> bool;

    /// Focuses the widget.
    fn focus(&mut self);

    /// Unfocuses the widget.
    fn unfocus(&mut self);
}

/// A summary of the focusable widgets present on a widget tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Count {
    /// The index of the current focused widget, if any.
    pub focused: Option<usize>,

    /// The total amount of focusable widgets.
    pub total: usize,
}

/// Produces an [`Operation`] that focuses the widget with the given [`Id`].
pub fn focus<T>(target: Id) -> impl Operation<T> {
    struct Focus {
        target: Id,
    }

    impl<T> Operation<T> for Focus {
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            match id {
                Some(id) if id == &self.target => {
                    state.focus();
                }
                _ => {
                    state.unfocus();
                }
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }
    }

    Focus { target }
}

/// Produces an [`Operation`] that unfocuses the focused widget.
pub fn unfocus<T>() -> impl Operation<T> {
    struct Unfocus;

    impl<T> Operation<T> for Unfocus {
        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            state.unfocus();
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }
    }

    Unfocus
}

/// Produces an [`Operation`] that generates a [`Count`] and chains it with the
/// provided function to build a new [`Operation`].
pub fn count() -> impl Operation<Count> {
    struct CountFocusable {
        count: Count,
    }

    impl Operation<Count> for CountFocusable {
        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if state.is_focused() {
                self.count.focused = Some(self.count.total);
            }

            self.count.total += 1;
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Count>)) {
            operate(self);
        }

        fn finish(&self) -> Outcome<Count> {
            Outcome::Some(self.count)
        }
    }

    CountFocusable {
        count: Count::default(),
    }
}

/// A [`Count`] paired with the [`Id`] of the scope it was computed within.
struct ScopedCountResult {
    target: Id,
    count: Count,
}

/// Produces an [`Operation`] that generates a [`Count`] of focusable widgets
/// within the container identified by `target`.
///
/// The result carries both the count and the target [`Id`] so that a
/// subsequent [`Operation`] can reuse it without capturing state.
fn scoped_count(target: Id) -> impl Operation<ScopedCountResult> {
    struct ScopedCount {
        target: Id,
        pending_scope: bool,
        inside_scope: bool,
        count: Count,
    }

    impl Operation<ScopedCountResult> for ScopedCount {
        fn container(&mut self, id: Option<&Id>, _bounds: Rectangle) {
            if id.is_some_and(|id| *id == self.target) {
                self.pending_scope = true;
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<ScopedCountResult>)) {
            let was_inside = self.inside_scope;
            if self.pending_scope {
                self.inside_scope = true;
                self.pending_scope = false;
            }
            operate(self);
            self.inside_scope = was_inside;
        }

        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if !self.inside_scope {
                return;
            }

            if state.is_focused() {
                self.count.focused = Some(self.count.total);
            }

            self.count.total += 1;
        }

        fn finish(&self) -> Outcome<ScopedCountResult> {
            Outcome::Some(ScopedCountResult {
                target: self.target.clone(),
                count: self.count,
            })
        }
    }

    ScopedCount {
        target,
        pending_scope: false,
        inside_scope: false,
        count: Count::default(),
    }
}

/// Produces an [`Operation`] that searches for the current focused widget, and
/// - if found, focuses the previous focusable widget.
/// - if not found, focuses the last focusable widget.
pub fn focus_previous<T>() -> impl Operation<T>
where
    T: Send + 'static,
{
    struct FocusPrevious {
        count: Count,
        current: usize,
    }

    impl<T> Operation<T> for FocusPrevious {
        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if self.count.total == 0 {
                return;
            }

            match self.count.focused {
                None if self.current == self.count.total - 1 => state.focus(),
                Some(0) if self.current == self.count.total - 1 => {
                    state.focus();
                }
                Some(0) if self.current == 0 => state.unfocus(),
                Some(0) => {}
                Some(focused) if focused == self.current => state.unfocus(),
                Some(focused) if focused - 1 == self.current => state.focus(),
                _ => {}
            }

            self.current += 1;
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }
    }

    operation::then(count(), |count| FocusPrevious { count, current: 0 })
}

/// Produces an [`Operation`] that searches for the current focused widget, and
/// - if found, focuses the next focusable widget.
/// - if not found, focuses the first focusable widget.
pub fn focus_next<T>() -> impl Operation<T>
where
    T: Send + 'static,
{
    struct FocusNext {
        count: Count,
        current: usize,
    }

    impl<T> Operation<T> for FocusNext {
        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            match self.count.focused {
                None if self.current == 0 => state.focus(),
                Some(focused) if focused == self.count.total - 1 && self.current == 0 => {
                    state.focus();
                }
                Some(focused) if focused == self.current => state.unfocus(),
                Some(focused) if focused + 1 == self.current => state.focus(),
                _ => {}
            }

            self.current += 1;
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }
    }

    operation::then(count(), |count| FocusNext { count, current: 0 })
}

/// Produces an [`Operation`] that cycles focus to the previous focusable
/// widget within the container identified by `target`.
///
/// Behaves like [`focus_previous`] but only considers widgets that are
/// descendants of `target`. Widgets outside the scope are not counted and
/// their focus state is not changed.
pub fn focus_previous_within<T>(target: Id) -> impl Operation<T>
where
    T: Send + 'static,
{
    struct ScopedFocusPrevious {
        target: Id,
        pending_scope: bool,
        inside_scope: bool,
        count: Count,
        current: usize,
    }

    impl<T> Operation<T> for ScopedFocusPrevious {
        fn container(&mut self, id: Option<&Id>, _bounds: Rectangle) {
            if id.is_some_and(|id| *id == self.target) {
                self.pending_scope = true;
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            let was_inside = self.inside_scope;
            if self.pending_scope {
                self.inside_scope = true;
                self.pending_scope = false;
            }
            operate(self);
            self.inside_scope = was_inside;
        }

        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if !self.inside_scope {
                return;
            }

            if self.count.total == 0 {
                return;
            }

            match self.count.focused {
                None if self.current == self.count.total - 1 => state.focus(),
                Some(0) if self.current == self.count.total - 1 => {
                    state.focus();
                }
                Some(0) if self.current == 0 => state.unfocus(),
                Some(0) => {}
                Some(focused) if focused == self.current => state.unfocus(),
                Some(focused) if focused - 1 == self.current => state.focus(),
                _ => {}
            }

            self.current += 1;
        }
    }

    operation::then(scoped_count(target), |result| ScopedFocusPrevious {
        target: result.target,
        pending_scope: false,
        inside_scope: false,
        count: result.count,
        current: 0,
    })
}

/// Produces an [`Operation`] that cycles focus to the next focusable widget
/// within the container identified by `target`.
///
/// Behaves like [`focus_next`] but only considers widgets that are
/// descendants of `target`. Widgets outside the scope are not counted and
/// their focus state is not changed.
pub fn focus_next_within<T>(target: Id) -> impl Operation<T>
where
    T: Send + 'static,
{
    struct ScopedFocusNext {
        target: Id,
        pending_scope: bool,
        inside_scope: bool,
        count: Count,
        current: usize,
    }

    impl<T> Operation<T> for ScopedFocusNext {
        fn container(&mut self, id: Option<&Id>, _bounds: Rectangle) {
            if id.is_some_and(|id| *id == self.target) {
                self.pending_scope = true;
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            let was_inside = self.inside_scope;
            if self.pending_scope {
                self.inside_scope = true;
                self.pending_scope = false;
            }
            operate(self);
            self.inside_scope = was_inside;
        }

        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if !self.inside_scope {
                return;
            }

            match self.count.focused {
                None if self.current == 0 => state.focus(),
                Some(focused) if focused == self.count.total - 1 && self.current == 0 => {
                    state.focus();
                }
                Some(focused) if focused == self.current => state.unfocus(),
                Some(focused) if focused + 1 == self.current => state.focus(),
                _ => {}
            }

            self.current += 1;
        }
    }

    operation::then(scoped_count(target), |result| ScopedFocusNext {
        target: result.target,
        pending_scope: false,
        inside_scope: false,
        count: result.count,
        current: 0,
    })
}

/// Produces an [`Operation`] that searches for the current focused widget
/// and stores its ID. This ignores widgets that do not have an ID.
pub fn find_focused() -> impl Operation<Id> {
    struct FindFocused {
        focused: Option<Id>,
    }

    impl Operation<Id> for FindFocused {
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if state.is_focused() && id.is_some() {
                self.focused = id.cloned();
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Id>)) {
            operate(self);
        }

        fn finish(&self) -> Outcome<Id> {
            if let Some(id) = &self.focused {
                Outcome::Some(id.clone())
            } else {
                Outcome::None
            }
        }
    }

    FindFocused { focused: None }
}

/// Produces an [`Operation`] that searches for the focusable widget
/// and stores whether it is focused or not. This ignores widgets that
/// do not have an ID.
pub fn is_focused(target: Id) -> impl Operation<bool> {
    struct IsFocused {
        target: Id,
        is_focused: Option<bool>,
    }

    impl Operation<bool> for IsFocused {
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if id.is_some_and(|id| *id == self.target) {
                self.is_focused = Some(state.is_focused());
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<bool>)) {
            if self.is_focused.is_some() {
                return;
            }

            operate(self);
        }

        fn finish(&self) -> Outcome<bool> {
            self.is_focused.map_or(Outcome::None, Outcome::Some)
        }
    }

    IsFocused {
        target,
        is_focused: None,
    }
}

/// Positional information about a scrollable.
#[derive(Debug, Clone, Copy)]
struct ScrollableInfo {
    bounds: Rectangle,
    content_bounds: Rectangle,
    translation: Vector,
}

impl ScrollableInfo {
    /// Whether this scrollable can scroll further in the given direction.
    fn can_scroll(&self, action: ScrollAction) -> bool {
        let max_y = (self.content_bounds.height - self.bounds.height).max(0.0);
        let max_x = (self.content_bounds.width - self.bounds.width).max(0.0);

        match action {
            ScrollAction::PageDown | ScrollAction::LineDown => self.translation.y < max_y - 0.5,
            ScrollAction::PageUp | ScrollAction::LineUp => self.translation.y > 0.5,
            ScrollAction::LineRight | ScrollAction::PageRight => self.translation.x < max_x - 0.5,
            ScrollAction::LineLeft | ScrollAction::PageLeft => self.translation.x > 0.5,
            ScrollAction::Home => self.translation.y > 0.5 || self.translation.x > 0.5,
            ScrollAction::End => {
                self.translation.y < max_y - 0.5 || self.translation.x < max_x - 0.5
            }
            ScrollAction::ShiftHome => self.translation.x > 0.5,
            ScrollAction::ShiftEnd => self.translation.x < max_x - 0.5,
        }
    }
}

/// A scroll adjustment to apply to a specific scrollable.
#[derive(Debug, Clone, Copy)]
struct ScrollAdjustment {
    scrollable_bounds: Rectangle,
    offset: AbsoluteOffset<Option<f32>>,
}

/// Margin added when scrolling a focused widget into view. Ensures the
/// widget isn't flush against the scrollable edge or hidden behind a
/// scrollbar.
const SCROLL_MARGIN: f32 = 12.0;

/// Computes the scroll offset needed to bring `target` into view within
/// a scrollable defined by `sb` (bounds), `content_bounds`, and `t`
/// (current translation / scroll offset).
///
/// Subtracts scrollbar thickness from the visible area when scrollbars
/// are present (detected via content overflow). Adds [`SCROLL_MARGIN`]
/// around the target so it isn't flush against the viewport edge.
///
/// Returns `None` if `target` is already fully visible.
fn compute_scroll_to(
    sb: Rectangle,
    content_bounds: Rectangle,
    t: Vector,
    target: Rectangle,
) -> Option<AbsoluteOffset<Option<f32>>> {
    // Convert target position to content-space coordinates.
    // layout.bounds() in operate() returns absolute positions WITHOUT
    // scroll translation (translation is only applied during draw).
    // Subtracting the scrollable origin gives the content-space position.
    let cx = target.x - sb.x;
    let cy = target.y - sb.y;

    // Compute visible viewport dimensions. When content overflows on one
    // axis, a scrollbar appears on the OTHER axis and reduces the viewport.
    // Default scrollbar width in iced is 10px + margin; we use a
    // conservative estimate that covers common configurations.
    let scrollbar_reserved = 12.0;

    let has_h_scrollbar = content_bounds.width > sb.width;
    let has_v_scrollbar = content_bounds.height > sb.height;

    let visible_w = if has_v_scrollbar {
        sb.width - scrollbar_reserved
    } else {
        sb.width
    };
    let visible_h = if has_h_scrollbar {
        sb.height - scrollbar_reserved
    } else {
        sb.height
    };

    let mut offset_x = None;
    let mut offset_y = None;

    // Check if target is outside the visible viewport [t, t + visible].
    // cx/cy is the content-space position; t is the current scroll offset.
    if target.width >= visible_w || cx < t.x {
        offset_x = Some((cx - SCROLL_MARGIN).max(0.0));
    } else if cx + target.width > t.x + visible_w {
        offset_x = Some(cx + target.width - visible_w + SCROLL_MARGIN);
    }

    if target.height >= visible_h || cy < t.y {
        offset_y = Some((cy - SCROLL_MARGIN).max(0.0));
    } else if cy + target.height > t.y + visible_h {
        offset_y = Some(cy + target.height - visible_h + SCROLL_MARGIN);
    }

    if offset_x.is_some() || offset_y.is_some() {
        Some(AbsoluteOffset {
            x: offset_x,
            y: offset_y,
        })
    } else {
        None
    }
}

/// Produces an [`Operation`] that scrolls the currently focused widget into
/// view by adjusting any scrollable ancestors whose viewport does not fully
/// contain it.
///
/// Uses a cascade approach for nested scrollables: the innermost scrollable
/// targets the focused widget, each outer scrollable targets the next inner
/// scrollable's bounds. This ensures proper framing at each nesting level.
pub fn scroll_focused_into_view<T>() -> impl Operation<T>
where
    T: Send + 'static,
{
    struct FindFocusedScrollContext {
        pending_scrollable: Option<ScrollableInfo>,
        scrollable_stack: Vec<ScrollableInfo>,
        focused_bounds: Option<Rectangle>,
        focused_ancestors: Vec<ScrollableInfo>,
    }

    impl Operation<Vec<ScrollAdjustment>> for FindFocusedScrollContext {
        fn scrollable(
            &mut self,
            _id: Option<&Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            translation: Vector,
            _state: &mut dyn Scrollable,
        ) {
            self.pending_scrollable = Some(ScrollableInfo {
                bounds,
                content_bounds,
                translation,
            });
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Vec<ScrollAdjustment>>)) {
            if let Some(info) = self.pending_scrollable.take() {
                self.scrollable_stack.push(info);
            }

            let depth = self.scrollable_stack.len();
            operate(self);
            self.scrollable_stack.truncate(depth);
        }

        fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
            if state.is_focused() {
                self.focused_bounds = Some(bounds);
                self.focused_ancestors = self.scrollable_stack.clone();
            }
        }

        fn finish(&self) -> Outcome<Vec<ScrollAdjustment>> {
            let Some(focused) = self.focused_bounds else {
                return Outcome::None;
            };

            let mut adjustments = Vec::new();

            // Cascade: innermost scrollable targets the focused widget,
            // each outer scrollable targets the next inner scrollable.
            let mut target_bounds = focused;

            for ancestor in self.focused_ancestors.iter().rev() {
                if let Some(offset) = compute_scroll_to(
                    ancestor.bounds,
                    ancestor.content_bounds,
                    ancestor.translation,
                    target_bounds,
                ) {
                    adjustments.push(ScrollAdjustment {
                        scrollable_bounds: ancestor.bounds,
                        offset,
                    });
                }

                // Outer ancestors target this scrollable, not the focused widget
                target_bounds = ancestor.bounds;
            }

            if adjustments.is_empty() {
                Outcome::None
            } else {
                Outcome::Some(adjustments)
            }
        }
    }

    struct ApplyScrollAdjustments {
        adjustments: Vec<ScrollAdjustment>,
        applied: usize,
    }

    impl<T> Operation<T> for ApplyScrollAdjustments {
        fn scrollable(
            &mut self,
            _id: Option<&Id>,
            bounds: Rectangle,
            _content_bounds: Rectangle,
            _translation: Vector,
            state: &mut dyn Scrollable,
        ) {
            if let Some(adj) = self
                .adjustments
                .iter()
                .find(|a| a.scrollable_bounds == bounds)
            {
                state.scroll_to(adj.offset);
                self.applied += 1;
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            if self.applied < self.adjustments.len() {
                operate(self);
            }
        }
    }

    operation::then(
        FindFocusedScrollContext {
            pending_scrollable: None,
            scrollable_stack: Vec::new(),
            focused_bounds: None,
            focused_ancestors: Vec::new(),
        },
        |adjustments| ApplyScrollAdjustments {
            adjustments,
            applied: 0,
        },
    )
}

/// A keyboard scroll action to apply to a scrollable.
#[derive(Debug, Clone, Copy)]
pub enum ScrollAction {
    /// Scroll up by viewport height.
    PageUp,
    /// Scroll down by viewport height.
    PageDown,
    /// Scroll up by one line.
    LineUp,
    /// Scroll down by one line.
    LineDown,
    /// Scroll left by one line.
    LineLeft,
    /// Scroll right by one line.
    LineRight,
    /// Scroll to the start.
    Home,
    /// Scroll to the end.
    End,
    /// Scroll left by viewport width (Shift+Page Up).
    PageLeft,
    /// Scroll right by viewport width (Shift+Page Down).
    PageRight,
    /// Scroll to horizontal start (Shift+Home).
    ShiftHome,
    /// Scroll to horizontal end (Shift+End).
    ShiftEnd,
}

/// Result from phase 1 of [`scroll_focused_ancestor`].
struct ScrollTarget {
    scrollable_bounds: Rectangle,
    action: ScrollAction,
}

/// Produces an [`Operation`] that scrolls a scrollable related to the
/// currently focused widget by the given [`ScrollAction`].
///
/// Uses scroll bubbling: tries the innermost scrollable ancestor first,
/// bubbles to outer ancestors if at the scroll limit, and falls back to
/// any scrollable in the tree if no ancestor can scroll.
pub fn scroll_focused_ancestor<T>(action: ScrollAction) -> impl Operation<T>
where
    T: Send + 'static,
{
    struct FindTarget {
        action: ScrollAction,
        pending_scrollable: Option<ScrollableInfo>,
        scrollable_stack: Vec<ScrollableInfo>,
        focused_ancestors: Option<Vec<ScrollableInfo>>,
        all_scrollables: Vec<ScrollableInfo>,
    }

    impl Operation<ScrollTarget> for FindTarget {
        fn scrollable(
            &mut self,
            _id: Option<&Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            translation: Vector,
            _state: &mut dyn Scrollable,
        ) {
            let info = ScrollableInfo {
                bounds,
                content_bounds,
                translation,
            };

            self.pending_scrollable = Some(info);
            self.all_scrollables.push(info);
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<ScrollTarget>)) {
            if let Some(info) = self.pending_scrollable.take() {
                self.scrollable_stack.push(info);
            }

            let depth = self.scrollable_stack.len();
            operate(self);
            self.scrollable_stack.truncate(depth);
        }

        fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if state.is_focused() && self.focused_ancestors.is_none() {
                self.focused_ancestors = Some(self.scrollable_stack.clone());
            }
        }

        fn finish(&self) -> Outcome<ScrollTarget> {
            // Try ancestors innermost to outermost (scroll bubbling)
            if let Some(ancestors) = &self.focused_ancestors {
                for ancestor in ancestors.iter().rev() {
                    if ancestor.can_scroll(self.action) {
                        return Outcome::Some(ScrollTarget {
                            scrollable_bounds: ancestor.bounds,
                            action: self.action,
                        });
                    }
                }
            }

            // Fall back to any scrollable in the tree.
            // Reverse search order for upward/backward actions so
            // scrolling back through sibling scrollables is symmetric
            // with scrolling forward.
            let reverse = matches!(
                self.action,
                ScrollAction::PageUp
                    | ScrollAction::LineUp
                    | ScrollAction::LineLeft
                    | ScrollAction::Home
                    | ScrollAction::PageLeft
                    | ScrollAction::ShiftHome
            );

            let find = |scrollables: &[ScrollableInfo]| {
                scrollables.iter().position(|s| s.can_scroll(self.action))
            };

            let index = if reverse {
                // Search from the end: find last scrollable that can scroll
                self.all_scrollables
                    .iter()
                    .rposition(|s| s.can_scroll(self.action))
            } else {
                find(&self.all_scrollables)
            };

            if let Some(i) = index {
                return Outcome::Some(ScrollTarget {
                    scrollable_bounds: self.all_scrollables[i].bounds,
                    action: self.action,
                });
            }

            Outcome::None
        }
    }

    struct ApplyScroll {
        target: ScrollTarget,
    }

    impl<T> Operation<T> for ApplyScroll {
        fn scrollable(
            &mut self,
            _id: Option<&Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            _translation: Vector,
            state: &mut dyn Scrollable,
        ) {
            if bounds != self.target.scrollable_bounds {
                return;
            }

            // Default line height for framework-level keyboard scrolling.
            // The widget-level handler uses the renderer's text size instead.
            let line_height = 16.0;

            match self.target.action {
                ScrollAction::PageDown => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: 0.0,
                            y: bounds.height,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::PageUp => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: 0.0,
                            y: -bounds.height,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::LineDown => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: 0.0,
                            y: line_height,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::LineUp => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: 0.0,
                            y: -line_height,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::LineRight => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: line_height,
                            y: 0.0,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::LineLeft => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: -line_height,
                            y: 0.0,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::Home => {
                    let overflows_x = content_bounds.width > bounds.width;
                    let overflows_y = content_bounds.height > bounds.height;

                    state.snap_to(RelativeOffset {
                        x: if overflows_x && !overflows_y {
                            Some(0.0)
                        } else {
                            None
                        },
                        y: if overflows_y || !overflows_x {
                            Some(0.0)
                        } else {
                            None
                        },
                    });
                }
                ScrollAction::End => {
                    let overflows_x = content_bounds.width > bounds.width;
                    let overflows_y = content_bounds.height > bounds.height;

                    state.snap_to(RelativeOffset {
                        x: if overflows_x && !overflows_y {
                            Some(1.0)
                        } else {
                            None
                        },
                        y: if overflows_y || !overflows_x {
                            Some(1.0)
                        } else {
                            None
                        },
                    });
                }
                ScrollAction::PageRight => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: bounds.width,
                            y: 0.0,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::PageLeft => {
                    state.scroll_by(
                        AbsoluteOffset {
                            x: -bounds.width,
                            y: 0.0,
                        },
                        bounds,
                        content_bounds,
                    );
                }
                ScrollAction::ShiftHome => {
                    state.snap_to(RelativeOffset {
                        x: Some(0.0),
                        y: None,
                    });
                }
                ScrollAction::ShiftEnd => {
                    state.snap_to(RelativeOffset {
                        x: Some(1.0),
                        y: None,
                    });
                }
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }
    }

    operation::then(
        FindTarget {
            action,
            pending_scrollable: None,
            scrollable_stack: Vec::new(),
            focused_ancestors: None,
            all_scrollables: Vec::new(),
        },
        |target| ApplyScroll { target },
    )
}

/// The direction in which [`focus_peer_in_list`] cycles focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Move focus to the previous entry in the list.
    Previous,
    /// Move focus to the next entry in the list.
    Next,
}

/// Intermediate result from [`focus_peer_in_list`]'s lookup phase.
///
/// Carries the resolved peer [`Id`] (if one could be found) through to
/// the focus phase.
#[derive(Debug, Clone)]
struct PeerTarget {
    candidate: Option<Id>,
}

/// Produces an [`Operation`] that moves focus to the next or previous
/// [`Id`] in `ids`, relative to whichever entry currently owns focus.
///
/// Each id may be either:
/// - the [`Id`] of a focusable widget (matched directly), or
/// - the [`Id`] of a [`container`](Operation::container) whose focusable
///   descendants are treated as owning the slot.
///
/// This mirrors how ids are declared on the host tree: widgets that
/// wrap their internal focusable state in a titled container (e.g.
/// [`Radio`]) expose the container id to the outside world, while
/// widgets whose focusable is itself addressable (e.g. text input) use
/// that id directly.
///
/// [`Radio`]: crate::widget::operation::accessible::Role::RadioButton
///
/// Intended for ARIA-style peer navigation, such as standalone radio
/// buttons declaring a shared group via `a11y.radio_group`. Wraps on
/// both ends.
///
/// Does nothing when:
/// - `ids` is empty.
/// - No currently-focused widget is found, or the focused widget is
///   not a descendant of any slot in `ids`.
/// - The resolved peer slot has no focusable descendant in the current
///   tree (e.g. the widget is disabled or no longer mounted).
pub fn focus_peer_in_list<T>(ids: &[Id], direction: Direction) -> impl Operation<T> + use<T>
where
    T: Send + 'static,
{
    /// First phase: walk the tree, maintain a stack of currently-open
    /// slot indices, record which slot owns the focused widget, and
    /// track which slots are actually mounted in the tree (so the
    /// candidate selection can skip slots that aren't present).
    struct FindPeer {
        ids: Vec<Id>,
        direction: Direction,
        /// Slot recorded during the most recent matching `container`
        /// call, promoted to [`slot_stack`] by the next `traverse`.
        pending_slot: Option<usize>,
        /// Stack of slot indices for currently-open containers matching
        /// an entry in `ids`. The innermost entry wins when descendants
        /// announce a focused focusable without its own id.
        slot_stack: Vec<usize>,
        /// Index into `ids` for the slot owning the focused widget.
        focused_slot: Option<usize>,
        /// Which slots we've seen mounted in the tree (either as a
        /// direct focusable or a container wrapping a focusable).
        seen: Vec<bool>,
    }

    impl FindPeer {
        fn slot_for(&self, id: Option<&Id>) -> Option<usize> {
            id.and_then(|id| self.ids.iter().position(|peer| peer == id))
        }
    }

    impl Operation<PeerTarget> for FindPeer {
        fn container(&mut self, id: Option<&Id>, _bounds: Rectangle) {
            if let Some(slot) = self.slot_for(id) {
                self.pending_slot = Some(slot);
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<PeerTarget>)) {
            let pushed = self.pending_slot.take();
            if let Some(slot) = pushed {
                self.slot_stack.push(slot);
            }
            operate(self);
            if pushed.is_some() {
                let _ = self.slot_stack.pop();
            }
        }

        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            // Mark the owning slot as mounted, whether it matches by
            // the focusable's own id or by an enclosing container.
            if let Some(slot) = self.slot_for(id) {
                self.seen[slot] = true;
            }
            if let Some(&slot) = self.slot_stack.last() {
                self.seen[slot] = true;
            }

            if !state.is_focused() || self.focused_slot.is_some() {
                return;
            }

            if let Some(slot) = self.slot_for(id) {
                self.focused_slot = Some(slot);
            } else if let Some(&slot) = self.slot_stack.last() {
                self.focused_slot = Some(slot);
            }
        }

        fn finish(&self) -> Outcome<PeerTarget> {
            let total = self.ids.len();
            let candidate = self.focused_slot.and_then(|current| {
                if total == 0 {
                    return None;
                }

                // Step from `current` in the chosen direction looking
                // for the nearest mounted peer. Skip slots that are
                // not present in the tree (e.g. disabled or not
                // currently rendered). Wrap on both ends.
                let step = |idx: usize| -> usize {
                    match self.direction {
                        Direction::Next => (idx + 1) % total,
                        Direction::Previous => (idx + total - 1) % total,
                    }
                };

                let mut cursor = step(current);
                while cursor != current {
                    if self.seen[cursor] {
                        return Some(self.ids[cursor].clone());
                    }
                    cursor = step(cursor);
                }

                None
            });

            Outcome::Some(PeerTarget { candidate })
        }
    }

    /// Second phase: walk the tree, focus the first focusable
    /// descendant of the target slot, and unfocus every other
    /// focusable widget. Unlike [`focus`], this operation does not
    /// require the target widget to carry its own [`Id`]: it accepts
    /// any focusable whose ancestor container id matches the target.
    struct ApplyPeer {
        target: Option<Id>,
        /// Matches `pending_slot` on the first phase: captured in
        /// `container`, promoted by the next `traverse`.
        pending_in_slot: bool,
        /// Depth of nested matches for the target container id.
        in_slot_depth: usize,
        /// Set once we've focused a descendant of the target slot, to
        /// avoid focusing a second sibling focusable if one exists.
        focused_one: bool,
    }

    impl<T> Operation<T> for ApplyPeer
    where
        T: Send + 'static,
    {
        fn container(&mut self, id: Option<&Id>, _bounds: Rectangle) {
            if let (Some(id), Some(target)) = (id, &self.target)
                && id == target
            {
                self.pending_in_slot = true;
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            let entered = std::mem::take(&mut self.pending_in_slot);
            if entered {
                self.in_slot_depth += 1;
            }
            operate(self);
            if entered {
                self.in_slot_depth -= 1;
            }
        }

        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            let Some(target) = &self.target else {
                // No target resolved: leave focus state alone.
                return;
            };

            // Direct id match or first descendant of the target slot
            // becomes the new focus holder.
            let matches = id == Some(target) || self.in_slot_depth > 0;

            if matches && !self.focused_one {
                state.focus();
                self.focused_one = true;
            } else {
                state.unfocus();
            }
        }
    }

    let total = ids.len();

    operation::then(
        FindPeer {
            ids: ids.to_vec(),
            direction,
            pending_slot: None,
            slot_stack: Vec::new(),
            focused_slot: None,
            seen: vec![false; total],
        },
        |target| ApplyPeer {
            target: target.candidate,
            pending_in_slot: false,
            in_slot_depth: 0,
            focused_one: false,
        },
    )
}

/// Produces an [`Operation`] that finds the currently focused widget
/// and returns its declared `a11y.radio_group` peer list, if any.
///
/// The list is returned exactly as the widget declared it: no
/// filtering, no validation, and no check that the focused widget's
/// own [`Id`] appears in the list. Callers that intend to cycle focus
/// through the list should hand it to [`focus_peer_in_list`], which
/// performs its own tree-presence check.
pub fn find_focused_radio_group() -> impl Operation<Vec<Id>> {
    struct FindFocusedRadioGroup {
        last_accessible_bounds: Option<Rectangle>,
        last_accessible_peers: Option<Vec<Id>>,
        result: Option<Vec<Id>>,
    }

    impl Operation<Vec<Id>> for FindFocusedRadioGroup {
        fn accessible(&mut self, _id: Option<&Id>, bounds: Rectangle, accessible: &Accessible<'_>) {
            // A widget with accessibility metadata calls `accessible`
            // immediately before its sibling traits (`focusable`,
            // `text`, etc.), so we stash the most recent values and
            // correlate on the next `focusable` call.
            self.last_accessible_bounds = Some(bounds);
            self.last_accessible_peers = accessible.radio_group.map(<[Id]>::to_vec);
        }

        fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
            if !state.is_focused() || self.result.is_some() {
                return;
            }

            if self.last_accessible_bounds == Some(bounds)
                && let Some(peers) = &self.last_accessible_peers
            {
                self.result = Some(peers.clone());
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Vec<Id>>)) {
            if self.result.is_none() {
                operate(self);
            }
        }

        fn finish(&self) -> Outcome<Vec<Id>> {
            match &self.result {
                Some(peers) => Outcome::Some(peers.clone()),
                None => Outcome::None,
            }
        }
    }

    FindFocusedRadioGroup {
        last_accessible_bounds: None,
        last_accessible_peers: None,
        result: None,
    }
}

/// The result of a successful mnemonic lookup.
#[derive(Debug, Clone)]
pub struct MnemonicTarget {
    /// The bounding rectangle of the matched widget.
    pub bounds: Rectangle,
    /// The widget [`Id`], if it has one.
    pub id: Option<Id>,
}

/// Produces an [`Operation`] that walks the widget tree and finds the
/// first enabled widget whose [`mnemonic`] matches `key`
/// (case-insensitive).
///
/// [`mnemonic`]: Accessible::mnemonic
pub fn find_mnemonic(key: char) -> impl Operation<MnemonicTarget> {
    struct FindMnemonic {
        key: char,
        found: Option<MnemonicTarget>,
    }

    impl Operation<MnemonicTarget> for FindMnemonic {
        fn accessible(&mut self, id: Option<&Id>, bounds: Rectangle, accessible: &Accessible<'_>) {
            if self.found.is_some() {
                return;
            }

            if let Some(mnemonic) = accessible.mnemonic
                && mnemonic.eq_ignore_ascii_case(&self.key)
                && !accessible.disabled
            {
                self.found = Some(MnemonicTarget {
                    bounds,
                    id: id.cloned(),
                });
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<MnemonicTarget>)) {
            if self.found.is_none() {
                operate(self);
            }
        }

        fn finish(&self) -> Outcome<MnemonicTarget> {
            match &self.found {
                Some(target) => Outcome::Some(target.clone()),
                None => Outcome::None,
            }
        }
    }

    FindMnemonic { key, found: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Size};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(w, h))
    }

    fn vec2(x: f32, y: f32) -> Vector {
        Vector::new(x, y)
    }

    // All tests use content-space coordinates for target bounds, matching
    // what the real widget tree provides: layout.bounds() in operate()
    // returns positions WITHOUT scroll translation applied.

    #[test]
    fn both_scrollbars_reduce_visible_area() {
        // Content overflows both axes -> both scrollbars present.
        // Target at right edge: fits in full viewport but is hidden
        // behind the vertical scrollbar's reserved space.
        let sb = rect(0.0, 0.0, 400.0, 300.0);
        let content = rect(0.0, 0.0, 600.0, 800.0);

        // 400 - 12 (scrollbar) - 50 (target width) + 1 = just past the edge
        let target = rect(339.0, 50.0, 50.0, 40.0);
        let result = compute_scroll_to(sb, content, vec2(0.0, 0.0), target);

        assert!(
            result.is_some(),
            "should scroll when target is behind scrollbar"
        );
        assert!(result.unwrap().x.expect("horizontal scroll") > 0.0);
    }

    #[test]
    fn sequential_tab_forward_and_backward() {
        // Both-direction scrollable (400x200) with content (600x800).
        // Both scrollbars present. Buttons at content y=10, y=300, y=600.
        //
        // Target bounds are content-space positions (layout.bounds()
        // in operate() does NOT include scroll translation).

        let sb = rect(0.0, 0.0, 400.0, 200.0);
        let content = rect(0.0, 0.0, 600.0, 800.0);

        let btn1 = rect(50.0, 10.0, 100.0, 40.0);
        let btn2 = rect(50.0, 300.0, 100.0, 40.0);
        let btn3 = rect(50.0, 600.0, 100.0, 40.0);

        // Tab to btn1 at scroll=0: content y=10 is within [0, 188]
        let r1 = compute_scroll_to(sb, content, vec2(0.0, 0.0), btn1);
        assert!(r1.is_none(), "btn1 should be visible at scroll=0");

        // Tab to btn2: content y=300 is below [0, 188]
        let r2 = compute_scroll_to(sb, content, vec2(0.0, 0.0), btn2).unwrap();
        let scroll_y = r2.y.expect("should scroll to btn2");
        // trailing: 300 + 40 - 188 + 12 = 164
        assert!(
            (scroll_y - 164.0).abs() < 0.1,
            "btn2: expected ~164, got {scroll_y}"
        );

        // Tab to btn3: content y=600 is below [164, 352]
        let r3 = compute_scroll_to(sb, content, vec2(0.0, scroll_y), btn3).unwrap();
        let scroll_y = r3.y.expect("should scroll to btn3");
        // trailing: 600 + 40 - 188 + 12 = 464
        assert!(
            (scroll_y - 464.0).abs() < 0.1,
            "btn3: expected ~464, got {scroll_y}"
        );

        // Shift+Tab back to btn2: content y=300 is ABOVE [464, 652]
        let r4 = compute_scroll_to(sb, content, vec2(0.0, scroll_y), btn2).unwrap();
        let scroll_y = r4.y.expect("should scroll back to btn2");
        // leading: (300 - 12).max(0) = 288
        assert!(
            (scroll_y - 288.0).abs() < 0.1,
            "back to btn2: expected ~288, got {scroll_y}"
        );

        // Shift+Tab back to btn1: content y=10 is ABOVE [288, 476]
        let r5 = compute_scroll_to(sb, content, vec2(0.0, scroll_y), btn1).unwrap();
        let scroll_y = r5.y.expect("should scroll back to btn1");
        // leading: (10 - 12).max(0) = 0
        assert!(
            (scroll_y - 0.0).abs() < 0.1,
            "back to btn1: expected ~0, got {scroll_y}"
        );
    }

    // ---------------------------------------------------------------
    // focus_peer_in_list tests
    //
    // Operations are exercised directly by synthesising the container
    // / focusable / traverse call sequence a real widget tree would
    // produce. Each "radio slot" is modelled as a container with an
    // [`Id`] wrapping a single anonymous focusable, matching Plushie's
    // layout for standalone radios.
    // ---------------------------------------------------------------

    struct FakeFocusable {
        is_focused: bool,
    }

    impl Focusable for FakeFocusable {
        fn is_focused(&self) -> bool {
            self.is_focused
        }

        fn focus(&mut self) {
            self.is_focused = true;
        }

        fn unfocus(&mut self) {
            self.is_focused = false;
        }
    }

    /// A single radio slot: a named container wrapping one focusable.
    struct Slot {
        id: Id,
        state: FakeFocusable,
    }

    impl Slot {
        fn new(name: &str, focused: bool) -> Self {
            Self {
                id: Id::from(name.to_string()),
                state: FakeFocusable {
                    is_focused: focused,
                },
            }
        }

        /// Drive the operation for this slot, matching how the real
        /// container + radio pair would call into `operate`.
        fn drive<T>(&mut self, op: &mut dyn Operation<T>) {
            op.container(Some(&self.id), rect(0.0, 0.0, 10.0, 10.0));
            op.traverse(&mut |inner| {
                inner.focusable(None, rect(0.0, 0.0, 10.0, 10.0), &mut self.state);
            });
        }
    }

    /// Drive the operation over a slice of [`Slot`]s, with each slot
    /// presented in list order at the same nesting depth.
    fn drive_slots<T>(op: &mut dyn Operation<T>, slots: &mut [Slot]) {
        for slot in slots.iter_mut() {
            slot.drive(op);
        }
    }

    /// Run an [`Operation`] to completion, draining any chained phases
    /// the same way `runtime::keyboard::run_operation` does for real.
    fn run_to_completion(slots: &mut [Slot], mut op: Box<dyn Operation<()>>) {
        loop {
            drive_slots(&mut *op, slots);
            match op.finish() {
                Outcome::Chain(next) => op = next,
                _ => break,
            }
        }
    }

    fn focused_slot_names(slots: &[Slot]) -> Vec<String> {
        slots
            .iter()
            .filter(|s| s.state.is_focused)
            .map(|s| s.id.to_string())
            .collect()
    }

    #[test]
    fn focus_peer_in_list_next_moves_to_next_peer() {
        let ids = vec![
            Id::from("r1".to_string()),
            Id::from("r2".to_string()),
            Id::from("r3".to_string()),
        ];
        let mut slots = vec![
            Slot::new("r1", true),
            Slot::new("r2", false),
            Slot::new("r3", false),
        ];

        let op: Box<dyn Operation<()>> = Box::new(focus_peer_in_list::<()>(&ids, Direction::Next));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r2"]);
    }

    #[test]
    fn focus_peer_in_list_previous_moves_to_previous_peer() {
        let ids = vec![
            Id::from("r1".to_string()),
            Id::from("r2".to_string()),
            Id::from("r3".to_string()),
        ];
        let mut slots = vec![
            Slot::new("r1", false),
            Slot::new("r2", true),
            Slot::new("r3", false),
        ];

        let op: Box<dyn Operation<()>> =
            Box::new(focus_peer_in_list::<()>(&ids, Direction::Previous));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r1"]);
    }

    #[test]
    fn focus_peer_in_list_wraps_forward_from_last_to_first() {
        let ids = vec![
            Id::from("r1".to_string()),
            Id::from("r2".to_string()),
            Id::from("r3".to_string()),
        ];
        let mut slots = vec![
            Slot::new("r1", false),
            Slot::new("r2", false),
            Slot::new("r3", true),
        ];

        let op: Box<dyn Operation<()>> = Box::new(focus_peer_in_list::<()>(&ids, Direction::Next));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r1"]);
    }

    #[test]
    fn focus_peer_in_list_wraps_backward_from_first_to_last() {
        let ids = vec![
            Id::from("r1".to_string()),
            Id::from("r2".to_string()),
            Id::from("r3".to_string()),
        ];
        let mut slots = vec![
            Slot::new("r1", true),
            Slot::new("r2", false),
            Slot::new("r3", false),
        ];

        let op: Box<dyn Operation<()>> =
            Box::new(focus_peer_in_list::<()>(&ids, Direction::Previous));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r3"]);
    }

    #[test]
    fn focus_peer_in_list_no_op_when_focused_widget_not_in_list() {
        // r2 is focused but is not declared as a peer of r1 / r3, so
        // arrow navigation should not move focus.
        let ids = vec![Id::from("r1".to_string()), Id::from("r3".to_string())];
        let mut slots = vec![
            Slot::new("r1", false),
            Slot::new("r2", true),
            Slot::new("r3", false),
        ];

        let op: Box<dyn Operation<()>> = Box::new(focus_peer_in_list::<()>(&ids, Direction::Next));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r2"]);
    }

    #[test]
    fn focus_peer_in_list_no_op_when_list_empty() {
        let ids: Vec<Id> = Vec::new();
        let mut slots = vec![
            Slot::new("r1", true),
            Slot::new("r2", false),
            Slot::new("r3", false),
        ];

        let op: Box<dyn Operation<()>> = Box::new(focus_peer_in_list::<()>(&ids, Direction::Next));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r1"]);
    }

    #[test]
    fn focus_peer_in_list_no_op_when_list_has_single_peer() {
        // Single-item list: next and previous both wrap back to the
        // same slot, which the operation treats as a no-op.
        let ids = vec![Id::from("r1".to_string())];
        let mut slots = vec![
            Slot::new("r1", true),
            Slot::new("r2", false),
            Slot::new("r3", false),
        ];

        let op: Box<dyn Operation<()>> = Box::new(focus_peer_in_list::<()>(&ids, Direction::Next));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r1"]);
    }

    #[test]
    fn focus_peer_in_list_no_op_when_resolved_peer_absent_from_tree() {
        // r1 is focused, r2 is declared as its peer, but r2 is not
        // actually mounted. Focus should not move (and nothing should
        // get mistakenly unfocused either).
        let ids = vec![Id::from("r1".to_string()), Id::from("r2".to_string())];
        let mut slots = vec![Slot::new("r1", true), Slot::new("r3", false)];

        let op: Box<dyn Operation<()>> = Box::new(focus_peer_in_list::<()>(&ids, Direction::Next));
        run_to_completion(&mut slots, op);

        assert_eq!(focused_slot_names(&slots), vec!["r1"]);
    }
}
