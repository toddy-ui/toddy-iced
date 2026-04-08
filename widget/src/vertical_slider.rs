//! Sliders let users set a value by moving an indicator.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::slider;
//!
//! struct State {
//!    value: f32,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     ValueChanged(f32),
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     slider(0.0..=100.0, state.value, Message::ValueChanged).into()
//! }
//!
//! fn update(state: &mut State, message: Message) {
//!     match message {
//!         Message::ValueChanged(value) => {
//!             state.value = value;
//!         }
//!     }
//! }
//! ```
use std::ops::RangeInclusive;

pub use crate::slider::{Handle, HandleShape, Rail, Style};

use crate::core::border::Border;
use crate::core::keyboard;
use crate::core::keyboard::key::{self, Key};
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::renderer;
use crate::core::theme::palette;
use crate::core::touch;
use crate::core::widget::Operation;
use crate::core::widget::operation::accessible::{Accessible, Orientation, Role, Value};
use crate::core::widget::operation::focusable::Focusable;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    self, Color, Element, Event, Length, Pixels, Point, Rectangle, Shadow, Shell, Size, Theme,
    Widget,
};

/// An vertical bar and a handle that selects a single value from a range of
/// values.
///
/// A [`VerticalSlider`] will try to fill the vertical space of its container.
///
/// The [`VerticalSlider`] range of numeric values is generic and its step size defaults
/// to 1 unit.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use iced::widget::vertical_slider;
///
/// struct State {
///    value: f32,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     ValueChanged(f32),
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     vertical_slider(0.0..=100.0, state.value, Message::ValueChanged).into()
/// }
///
/// fn update(state: &mut State, message: Message) {
///     match message {
///         Message::ValueChanged(value) => {
///             state.value = value;
///         }
///     }
/// }
/// ```
pub struct VerticalSlider<'a, T, Message, Theme = crate::Theme>
where
    Theme: Catalog,
{
    range: RangeInclusive<T>,
    step: T,
    shift_step: Option<T>,
    value: T,
    default: Option<T>,
    on_change: Box<dyn Fn(T) -> Message + 'a>,
    on_release: Option<Message>,
    width: f32,
    height: Length,
    label: Option<String>,
    on_status_change: Option<Box<dyn Fn(&str) -> Message + 'a>>,
    class: Theme::Class<'a>,
    status: Option<Status>,
}

impl<'a, T, Message, Theme> VerticalSlider<'a, T, Message, Theme>
where
    T: Copy + From<u8> + std::cmp::PartialOrd,
    Message: Clone,
    Theme: Catalog,
{
    /// The default width of a [`VerticalSlider`].
    pub const DEFAULT_WIDTH: f32 = 16.0;

    /// Creates a new [`VerticalSlider`].
    ///
    /// It expects:
    ///   * an inclusive range of possible values
    ///   * the current value of the [`VerticalSlider`]
    ///   * a function that will be called when the [`VerticalSlider`] is dragged.
    ///     It receives the new value of the [`VerticalSlider`] and must produce a
    ///     `Message`.
    pub fn new<F>(range: RangeInclusive<T>, value: T, on_change: F) -> Self
    where
        F: 'a + Fn(T) -> Message,
    {
        let value = if value >= *range.start() {
            value
        } else {
            *range.start()
        };

        let value = if value <= *range.end() {
            value
        } else {
            *range.end()
        };

        VerticalSlider {
            value,
            default: None,
            range,
            step: T::from(1),
            shift_step: None,
            on_change: Box::new(on_change),
            on_release: None,
            width: Self::DEFAULT_WIDTH,
            height: Length::Fill,
            label: None,
            on_status_change: None,
            class: Theme::default(),
            status: None,
        }
    }

    /// Sets the optional default value for the [`VerticalSlider`].
    ///
    /// If set, the [`VerticalSlider`] will reset to this value when ctrl-clicked or command-clicked.
    pub fn default(mut self, default: impl Into<T>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Sets the release message of the [`VerticalSlider`].
    /// This is called when the mouse is released from the slider.
    ///
    /// Typically, the user's interaction with the slider is finished when this message is produced.
    /// This is useful if you need to spawn a long-running task from the slider's result, where
    /// the default on_change message could create too many events.
    pub fn on_release(mut self, on_release: Message) -> Self {
        self.on_release = Some(on_release);
        self
    }

    /// Sets the width of the [`VerticalSlider`].
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into().0;
        self
    }

    /// Sets the height of the [`VerticalSlider`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the step size of the [`VerticalSlider`].
    pub fn step(mut self, step: T) -> Self {
        self.step = step;
        self
    }

    /// Sets the optional "shift" step for the [`VerticalSlider`].
    ///
    /// If set, this value is used as the step while the shift key is pressed.
    pub fn shift_step(mut self, shift_step: impl Into<T>) -> Self {
        self.shift_step = Some(shift_step.into());
        self
    }

    /// Sets the accessible label for the [`VerticalSlider`].
    ///
    /// This is announced by screen readers as the name of the slider.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the callback for status change notifications.
    ///
    /// The callback receives the new status name as a string
    /// (e.g. "active", "hovered", "dragged", "focused").
    pub fn on_status_change(
        mut self,
        f: impl Fn(&str) -> Message + 'a,
    ) -> Self {
        self.on_status_change = Some(Box::new(f));
        self
    }

    /// Sets the style of the [`VerticalSlider`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`VerticalSlider`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    is_dragging: bool,
    is_focused: bool,
    focus_visible: bool,
    keyboard_modifiers: keyboard::Modifiers,
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
        self.focus_visible = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
        self.focus_visible = false;
    }
}

impl<T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VerticalSlider<'_, T, Message, Theme>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive,
    Message: Clone,
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();

        operation.accessible(
            None,
            layout.bounds(),
            &Accessible {
                role: Role::Slider,
                label: self.label.as_deref(),
                value: Some(Value::Numeric {
                    current: self.value.into(),
                    min: (*self.range.start()).into(),
                    max: (*self.range.end()).into(),
                    step: Some(self.step.into()),
                }),
                orientation: Some(Orientation::Vertical),
                // Set aria-busy during drag so assistive technology
                // suppresses rapid value announcements and announces
                // only the final value on release. See WAI-ARIA
                // aria-busy specification.
                busy: state.is_dragging,
                ..Accessible::default()
            },
        );

        operation.focusable(None, layout.bounds(), state);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let is_dragging = state.is_dragging;
        let current_value = self.value;

        let locate = |cursor_position: Point| -> Option<T> {
            let bounds = layout.bounds();

            if cursor_position.y >= bounds.y + bounds.height {
                Some(*self.range.start())
            } else if cursor_position.y <= bounds.y {
                Some(*self.range.end())
            } else {
                let step = if state.keyboard_modifiers.shift() {
                    self.shift_step.unwrap_or(self.step)
                } else {
                    self.step
                }
                .into();

                let start = (*self.range.start()).into();
                let end = (*self.range.end()).into();

                let percent =
                    1.0 - f64::from(cursor_position.y - bounds.y) / f64::from(bounds.height);

                let steps = (percent * (end - start) / step).round();
                let value = steps * step + start;

                T::from_f64(value.min(end))
            }
        };

        let increment = |value: T| -> Option<T> {
            let step = if state.keyboard_modifiers.shift() {
                self.shift_step.unwrap_or(self.step)
            } else {
                self.step
            }
            .into();

            let steps = (value.into() / step).round();
            let new_value = step * (steps + 1.0);

            if new_value > (*self.range.end()).into() {
                return Some(*self.range.end());
            }

            T::from_f64(new_value)
        };

        let decrement = |value: T| -> Option<T> {
            let step = if state.keyboard_modifiers.shift() {
                self.shift_step.unwrap_or(self.step)
            } else {
                self.step
            }
            .into();

            let steps = (value.into() / step).round();
            let new_value = step * (steps - 1.0);

            if new_value < (*self.range.start()).into() {
                return Some(*self.range.start());
            }

            T::from_f64(new_value)
        };

        let page_increment = |value: T| -> Option<T> {
            let step = self.step.into() * 10.0;
            let new_value = value.into() + step;

            if new_value > (*self.range.end()).into() {
                return Some(*self.range.end());
            }

            T::from_f64(new_value)
        };

        let page_decrement = |value: T| -> Option<T> {
            let step = self.step.into() * 10.0;
            let new_value = value.into() - step;

            if new_value < (*self.range.start()).into() {
                return Some(*self.range.start());
            }

            T::from_f64(new_value)
        };

        let mut change = |new_value: T| {
            if (self.value.into() - new_value.into()).abs() > f64::EPSILON {
                shell.publish((self.on_change)(new_value));

                self.value = new_value;
            }
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor_position) = cursor.position_over(layout.bounds()) {
                    if state.keyboard_modifiers.control() || state.keyboard_modifiers.command() {
                        let _ = self.default.map(change);
                        state.is_dragging = false;
                    } else {
                        let _ = locate(cursor_position).map(change);
                        state.is_dragging = true;
                    }

                    state.is_focused = true;
                    state.focus_visible = false;

                    shell.capture_event();
                } else {
                    state.is_focused = false;
                    state.focus_visible = false;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                if is_dragging {
                    if let Some(on_release) = self.on_release.clone() {
                        shell.publish(on_release);
                    }
                    state.is_dragging = false;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => {
                if is_dragging {
                    let _ = cursor.land().position().and_then(locate).map(change);

                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta })
                if state.keyboard_modifiers.control() =>
            {
                if cursor.is_over(layout.bounds()) {
                    let delta = match *delta {
                        mouse::ScrollDelta::Lines { x: _, y } => y,
                        mouse::ScrollDelta::Pixels { x: _, y } => y,
                    };

                    if delta < 0.0 {
                        let _ = decrement(current_value).map(change);
                    } else {
                        let _ = increment(current_value).map(change);
                    }

                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if cursor.is_over(layout.bounds()) || state.is_focused {
                    match key {
                        Key::Named(key::Named::ArrowUp | key::Named::ArrowRight) => {
                            let _ = increment(current_value).map(change);
                            shell.capture_event();
                        }
                        Key::Named(key::Named::ArrowDown | key::Named::ArrowLeft) => {
                            let _ = decrement(current_value).map(change);
                            shell.capture_event();
                        }
                        Key::Named(key::Named::PageUp) => {
                            let _ = page_increment(current_value).map(change);
                            shell.capture_event();
                        }
                        Key::Named(key::Named::PageDown) => {
                            let _ = page_decrement(current_value).map(change);
                            shell.capture_event();
                        }
                        Key::Named(key::Named::Home) => {
                            change(*self.range.start());
                            shell.capture_event();
                        }
                        Key::Named(key::Named::End) => {
                            change(*self.range.end());
                            shell.capture_event();
                        }
                        Key::Named(key::Named::Escape) => {
                            if state.is_focused {
                                state.is_focused = false;
                                state.focus_visible = false;
                                shell.capture_event();
                            }
                        }
                        _ => (),
                    }
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = *modifiers;
            }
            _ => {}
        }

        let current_status = if state.is_dragging {
            Status::Dragged
        } else if state.focus_visible {
            Status::Focused
        } else if cursor.is_over(layout.bounds()) {
            Status::Hovered
        } else {
            Status::Active
        };

        let new_name = status_name(&current_status);
        let old_name = self.status.as_ref().map(status_name);
        if old_name != Some(new_name) {
            if let Some(ref on_status_change) = self.on_status_change {
                shell.publish(on_status_change(new_name));
            }
        }

        if self.status.is_some_and(|s| s != current_status)
            || self.status.is_none()
        {
            if !matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
                shell.request_redraw();
            }
        }
        self.status = Some(current_status);
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let style = theme.style(&self.class, self.status.unwrap_or(Status::Active));

        let (handle_width, handle_height, handle_border_radius) = match style.handle.shape {
            HandleShape::Circle { radius } => (radius * 2.0, radius * 2.0, radius.into()),
            HandleShape::Rectangle {
                width,
                border_radius,
            } => (f32::from(width), bounds.width, border_radius),
        };

        let value = self.value.into() as f32;
        let (range_start, range_end) = {
            let (start, end) = self.range.clone().into_inner();

            (start.into() as f32, end.into() as f32)
        };

        let offset = if range_start >= range_end {
            0.0
        } else {
            (bounds.height - handle_width) * (value - range_end) / (range_start - range_end)
        };

        let rail_x = bounds.x + bounds.width / 2.0;

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: rail_x - style.rail.width / 2.0,
                    y: bounds.y,
                    width: style.rail.width,
                    height: offset + handle_width / 2.0,
                },
                border: style.rail.border,
                ..renderer::Quad::default()
            },
            style.rail.backgrounds.1,
        );

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: rail_x - style.rail.width / 2.0,
                    y: bounds.y + offset + handle_width / 2.0,
                    width: style.rail.width,
                    height: bounds.height - offset - handle_width / 2.0,
                },
                border: style.rail.border,
                ..renderer::Quad::default()
            },
            style.rail.backgrounds.0,
        );

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: rail_x - handle_height / 2.0,
                    y: bounds.y + offset,
                    width: handle_height,
                    height: handle_width,
                },
                border: Border {
                    radius: handle_border_radius,
                    width: style.handle.border_width,
                    color: style.handle.border_color,
                },
                shadow: style.handle.shadow,
                ..renderer::Quad::default()
            },
            style.handle.background,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.is_dragging {
            // FIXME: Fall back to `Pointer` on Windows
            // See https://github.com/rust-windowing/winit/issues/1043
            if cfg!(target_os = "windows") {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Grabbing
            }
        } else if cursor.is_over(layout.bounds()) {
            if cfg!(target_os = "windows") {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Grab
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, T, Message, Theme, Renderer> From<VerticalSlider<'a, T, Message, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive + 'a,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: core::Renderer + 'a,
{
    fn from(
        slider: VerticalSlider<'a, T, Message, Theme>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(slider)
    }
}

/// The possible status of a [`VerticalSlider`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`VerticalSlider`] can be interacted with.
    Active,
    /// The [`VerticalSlider`] is being hovered.
    Hovered,
    /// The [`VerticalSlider`] is being dragged.
    Dragged,
    /// The [`VerticalSlider`] has keyboard focus.
    Focused,
}

fn status_name(status: &Status) -> &'static str {
    match status {
        Status::Active => "active",
        Status::Hovered => "hovered",
        Status::Dragged => "dragged",
        Status::Focused => "focused",
    }
}

/// The theme catalog of a [`VerticalSlider`].
pub trait Catalog: Sized {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`VerticalSlider`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a [`VerticalSlider`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let color = match status {
        Status::Active => palette.primary.base.color,
        Status::Hovered => palette.primary.strong.color,
        Status::Dragged => palette.primary.weak.color,
        Status::Focused => palette.primary.strong.color,
    };

    Style {
        rail: Rail {
            backgrounds: (color.into(), palette.background.strong.color.into()),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 7.0 },
            background: color.into(),
            border_color: match status {
                Status::Focused => {
                    let accent = palette.primary.strong.color;
                    let page_bg = palette.background.base.color;
                    palette::focus_border_color(color, accent, page_bg)
                }
                _ => Color::TRANSPARENT,
            },
            border_width: match status {
                Status::Focused => 2.0,
                _ => 0.0,
            },
            shadow: match status {
                Status::Focused => palette::focus_shadow(
                    palette.primary.strong.color,
                    palette.background.base.color,
                ),
                _ => Shadow::default(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::widget::operation::focusable::Focusable;

    #[test]
    fn focusable_trait() {
        let mut state = State::default();
        assert!(!state.is_focused());
        assert!(!state.focus_visible);
        state.focus();
        assert!(state.is_focused());
        assert!(state.focus_visible);
        state.unfocus();
        assert!(!state.is_focused());
        assert!(!state.focus_visible);
    }

    #[test]
    fn default_state_not_focused() {
        let state = State::default();
        assert!(!state.is_focused);
        assert!(!state.is_dragging);
        assert!(!state.focus_visible);
    }

    #[test]
    fn focus_independent_of_drag() {
        let mut state = State::default();

        state.focus();
        assert!(!state.is_dragging);

        state.is_dragging = true;
        assert!(state.is_focused());

        state.unfocus();
        assert!(state.is_dragging);
    }
}
