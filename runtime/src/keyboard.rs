//! Track keyboard events and handle keyboard navigation.
pub use iced_core::keyboard::*;

use crate::UserInterface;
use crate::core;
use crate::core::widget::operation::{self, Operation as _};

/// Runs a chained operation to completion.
fn run_operation<Message, Theme, Renderer>(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
    mut op: Box<dyn operation::Operation>,
) where
    Renderer: core::Renderer,
{
    loop {
        ui.operate(renderer, op.as_mut());

        match op.finish() {
            operation::Outcome::Chain(next) => {
                op = next;
            }
            _ => break,
        }
    }
}

/// Moves focus and scrolls the newly focused widget into view.
fn focus_and_scroll<Message, Theme, Renderer>(
    shift: bool,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
) where
    Renderer: core::Renderer,
{
    let op: Box<dyn operation::Operation> = if shift {
        Box::new(operation::focusable::focus_previous::<()>())
    } else {
        Box::new(operation::focusable::focus_next::<()>())
    };

    run_operation(ui, renderer, op);

    run_operation(
        ui,
        renderer,
        Box::new(operation::focusable::scroll_focused_into_view::<()>()),
    );
}

/// Moves focus within a scope and scrolls the newly focused widget into view.
fn focus_and_scroll_within<Message, Theme, Renderer>(
    shift: bool,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
    scope: core::widget::Id,
) where
    Renderer: core::Renderer,
{
    let op: Box<dyn operation::Operation> = if shift {
        Box::new(operation::focusable::focus_previous_within::<()>(scope))
    } else {
        Box::new(operation::focusable::focus_next_within::<()>(scope))
    };

    run_operation(ui, renderer, op);

    run_operation(
        ui,
        renderer,
        Box::new(operation::focusable::scroll_focused_into_view::<()>()),
    );
}

/// Handle Ctrl+Tab / Ctrl+Shift+Tab for unconditional focus navigation.
///
/// This always moves focus regardless of whether any widget captured
/// the event. It is the emergency exit from any focus trap.
///
/// Returns `true` if the event was consumed.
pub fn handle_ctrl_tab<Message, Theme, Renderer>(
    event: &core::Event,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> bool
where
    Renderer: core::Renderer,
{
    if let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Named(core::keyboard::key::Named::Tab),
        modifiers,
        ..
    }) = event
        && modifiers.control()
    {
        focus_and_scroll(modifiers.shift(), ui, renderer);
        return true;
    }

    false
}

/// Handle uncaptured Tab / Shift+Tab for focus navigation.
///
/// Moves focus to the next (or previous) focusable widget and scrolls
/// it into view. Only runs when the event was not captured by any widget.
///
/// Returns `true` if the event was consumed.
pub fn handle_tab<Message, Theme, Renderer>(
    event: &core::Event,
    status: core::event::Status,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> bool
where
    Renderer: core::Renderer,
{
    if status != core::event::Status::Ignored {
        return false;
    }

    if let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Named(core::keyboard::key::Named::Tab),
        modifiers,
        ..
    }) = event
    {
        focus_and_scroll(modifiers.shift(), ui, renderer);
        return true;
    }

    false
}

/// Handle Ctrl+Tab / Ctrl+Shift+Tab for focus navigation within a scope.
///
/// Behaves like [`handle_ctrl_tab`] but restricts focus cycling to widgets
/// that are descendants of the container with the given `scope` [`Id`](core::widget::Id).
///
/// Returns `true` if the event was consumed.
pub fn handle_ctrl_tab_within<Message, Theme, Renderer>(
    event: &core::Event,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
    scope: core::widget::Id,
) -> bool
where
    Renderer: core::Renderer,
{
    if let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Named(core::keyboard::key::Named::Tab),
        modifiers,
        ..
    }) = event
        && modifiers.control()
    {
        focus_and_scroll_within(modifiers.shift(), ui, renderer, scope);
        return true;
    }

    false
}

/// Handle uncaptured Tab / Shift+Tab for focus navigation within a scope.
///
/// Behaves like [`handle_tab`] but restricts focus cycling to widgets
/// that are descendants of the container with the given `scope` [`Id`](core::widget::Id).
///
/// Returns `true` if the event was consumed.
pub fn handle_tab_within<Message, Theme, Renderer>(
    event: &core::Event,
    status: core::event::Status,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
    scope: core::widget::Id,
) -> bool
where
    Renderer: core::Renderer,
{
    if status != core::event::Status::Ignored {
        return false;
    }

    if let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Named(core::keyboard::key::Named::Tab),
        modifiers,
        ..
    }) = event
    {
        focus_and_scroll_within(modifiers.shift(), ui, renderer, scope);
        return true;
    }

    false
}

/// Handle Alt+letter mnemonics for widget activation.
///
/// When the user presses Alt plus a letter key (without Ctrl or Super),
/// searches the widget tree for a widget whose [`mnemonic`] matches the
/// pressed character. If found and the widget has an [`Id`], it is
/// focused. Returns `Some(bounds)` with the matched widget's bounding
/// rectangle so the caller can generate a synthetic click, or `None` if
/// no match was found.
///
/// [`mnemonic`]: core::widget::operation::accessible::Accessible::mnemonic
/// [`Id`]: core::widget::Id
pub fn handle_mnemonic<Message, Theme, Renderer>(
    event: &core::Event,
    status: core::event::Status,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> Option<core::Rectangle>
where
    Renderer: core::Renderer,
{
    if status != core::event::Status::Ignored {
        return None;
    }

    if let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Character(smol),
        modifiers,
        ..
    }) = event
    {
        if !modifiers.alt() || modifiers.control() || modifiers.logo() {
            return None;
        }

        let ch = smol.chars().next()?;

        let mut op = operation::focusable::find_mnemonic(ch);
        ui.operate(renderer, &mut operation::black_box::<_, ()>(&mut op));

        if let operation::Outcome::Some(target) = op.finish() {
            if let Some(id) = target.id {
                run_operation(
                    ui,
                    renderer,
                    Box::new(operation::focusable::focus::<()>(id)),
                );
            }

            return Some(target.bounds);
        }
    }

    None
}

/// Handle plain arrow keys for radio-group peer cycling.
///
/// Implements the WAI-ARIA "radio" pattern for standalone radio
/// buttons declaring peers via `a11y.radio_group`:
/// - ArrowDown / ArrowRight moves focus to the next peer in the list.
/// - ArrowUp / ArrowLeft moves focus to the previous peer.
/// - Both directions wrap on the ends.
/// - Only plain arrows are consumed; arrow keys combined with any
///   modifier (Shift, Ctrl, Alt, or Meta / logo) fall through so that
///   scroll-key handling and other modifier-driven shortcuts still
///   work unchanged.
///
/// Does nothing and returns `false` when:
/// - The focused widget has no `a11y.radio_group` declared (or it's
///   empty).
/// - The focused widget is not itself a peer of any slot in its
///   declared list.
/// - The resolved peer slot is not currently mounted in the tree.
///
/// Returns `true` if focus moved (and the event should be treated as
/// consumed).
pub fn handle_radio_arrow<Message, Theme, Renderer>(
    event: &core::Event,
    status: core::event::Status,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> bool
where
    Renderer: core::Renderer,
{
    if status != core::event::Status::Ignored {
        return false;
    }

    let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Named(named),
        modifiers,
        ..
    }) = event
    else {
        return false;
    };

    if modifiers.shift() || modifiers.control() || modifiers.alt() || modifiers.logo() {
        return false;
    }

    let direction = match named {
        core::keyboard::key::Named::ArrowDown | core::keyboard::key::Named::ArrowRight => {
            operation::focusable::Direction::Next
        }
        core::keyboard::key::Named::ArrowUp | core::keyboard::key::Named::ArrowLeft => {
            operation::focusable::Direction::Previous
        }
        _ => return false,
    };

    // Phase 1: ask the tree for the focused widget's declared peer
    // list. If none, we have nothing to do.
    let mut find = operation::focusable::find_focused_radio_group();
    ui.operate(renderer, &mut operation::black_box::<_, ()>(&mut find));

    let peers = match find.finish() {
        operation::Outcome::Some(peers) if !peers.is_empty() => peers,
        _ => return false,
    };

    // Phase 2: move focus to the resolved peer.
    run_operation(
        ui,
        renderer,
        Box::new(operation::focusable::focus_peer_in_list::<()>(
            &peers, direction,
        )),
    );

    // Scroll the newly focused peer into view so long radio lists
    // behave like every other focus move.
    run_operation(
        ui,
        renderer,
        Box::new(operation::focusable::scroll_focused_into_view::<()>()),
    );

    true
}

/// Handle uncaptured scroll keys for focused ancestor scrolling.
///
/// Maps keyboard scroll keys (Page Up/Down, arrows, Home/End, and
/// their Shift variants for horizontal scrolling) to scroll actions
/// on the focused widget's nearest scrollable ancestor.
///
/// Returns `true` if the event was consumed.
pub fn handle_scroll_keys<Message, Theme, Renderer>(
    event: &core::Event,
    status: core::event::Status,
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> bool
where
    Renderer: core::Renderer,
{
    if status != core::event::Status::Ignored {
        return false;
    }

    if let core::Event::Keyboard(core::keyboard::Event::KeyPressed {
        key: core::keyboard::Key::Named(named),
        modifiers,
        ..
    }) = event
    {
        use operation::focusable::ScrollAction;

        let action = match named {
            core::keyboard::key::Named::PageDown if modifiers.shift() => {
                Some(ScrollAction::PageRight)
            }
            core::keyboard::key::Named::PageDown => Some(ScrollAction::PageDown),
            core::keyboard::key::Named::PageUp if modifiers.shift() => Some(ScrollAction::PageLeft),
            core::keyboard::key::Named::PageUp => Some(ScrollAction::PageUp),
            core::keyboard::key::Named::ArrowDown if modifiers.shift() => {
                Some(ScrollAction::LineRight)
            }
            core::keyboard::key::Named::ArrowDown => Some(ScrollAction::LineDown),
            core::keyboard::key::Named::ArrowUp if modifiers.shift() => {
                Some(ScrollAction::LineLeft)
            }
            core::keyboard::key::Named::ArrowUp => Some(ScrollAction::LineUp),
            core::keyboard::key::Named::ArrowRight => Some(ScrollAction::LineRight),
            core::keyboard::key::Named::ArrowLeft => Some(ScrollAction::LineLeft),
            core::keyboard::key::Named::Home if modifiers.shift() => Some(ScrollAction::ShiftHome),
            core::keyboard::key::Named::Home => Some(ScrollAction::Home),
            core::keyboard::key::Named::End if modifiers.shift() => Some(ScrollAction::ShiftEnd),
            core::keyboard::key::Named::End => Some(ScrollAction::End),
            _ => None,
        };

        if let Some(action) = action {
            run_operation(
                ui,
                renderer,
                Box::new(operation::focusable::scroll_focused_ancestor::<()>(action)),
            );
            return true;
        }
    }

    false
}
