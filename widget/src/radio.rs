//! Radio buttons let users choose a single option from a bunch of options.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::{column, radio};
//!
//! struct State {
//!    selection: Option<Choice>,
//! }
//!
//! #[derive(Debug, Clone, Copy)]
//! enum Message {
//!     RadioSelected(Choice),
//! }
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum Choice {
//!     A,
//!     B,
//!     C,
//!     All,
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     let a = radio(
//!         "A",
//!         Choice::A,
//!         state.selection,
//!         Message::RadioSelected,
//!     );
//!
//!     let b = radio(
//!         "B",
//!         Choice::B,
//!         state.selection,
//!         Message::RadioSelected,
//!     );
//!
//!     let c = radio(
//!         "C",
//!         Choice::C,
//!         state.selection,
//!         Message::RadioSelected,
//!     );
//!
//!     let all = radio(
//!         "All of the above",
//!         Choice::All,
//!         state.selection,
//!         Message::RadioSelected
//!     );
//!
//!     column![a, b, c, all].into()
//! }
//! ```
use crate::core::alignment;
use crate::core::border::{self, Border};
use crate::core::keyboard;
use crate::core::keyboard::key;
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::theme::palette;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::operation::accessible::{Accessible, Role};
use crate::core::widget::operation::focusable::Focusable;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Background, Color, Element, Event, Layout, Length, Pixels, Rectangle, Shadow, Shell, Size,
    Theme, Widget,
};

/// A circular button representing a choice.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use iced::widget::{column, radio};
///
/// struct State {
///    selection: Option<Choice>,
/// }
///
/// #[derive(Debug, Clone, Copy)]
/// enum Message {
///     RadioSelected(Choice),
/// }
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum Choice {
///     A,
///     B,
///     C,
///     All,
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     let a = radio(
///         "A",
///         Choice::A,
///         state.selection,
///         Message::RadioSelected,
///     );
///
///     let b = radio(
///         "B",
///         Choice::B,
///         state.selection,
///         Message::RadioSelected,
///     );
///
///     let c = radio(
///         "C",
///         Choice::C,
///         state.selection,
///         Message::RadioSelected,
///     );
///
///     let all = radio(
///         "All of the above",
///         Choice::All,
///         state.selection,
///         Message::RadioSelected
///     );
///
///     column![a, b, c, all].into()
/// }
/// ```
pub struct Radio<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    is_selected: bool,
    on_click: Message,
    label: String,
    width: Length,
    size: f32,
    spacing: f32,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
    shaping: text::Shaping,
    wrapping: text::Wrapping,
    font: Option<Renderer::Font>,
    on_status_change: Option<Box<dyn Fn(&str) -> Message + 'a>>,
    class: Theme::Class<'a>,
    last_status: Option<Status>,
}

impl<'a, Message, Theme, Renderer> Radio<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// The default size of a [`Radio`] button.
    pub const DEFAULT_SIZE: f32 = 16.0;

    /// The default spacing of a [`Radio`] button.
    pub const DEFAULT_SPACING: f32 = 8.0;

    /// Creates a new [`Radio`] button.
    ///
    /// It expects:
    ///   * the value related to the [`Radio`] button
    ///   * the label of the [`Radio`] button
    ///   * the current selected value
    ///   * a function that will be called when the [`Radio`] is selected. It
    ///     receives the value of the radio and must produce a `Message`.
    pub fn new<F, V>(label: impl Into<String>, value: V, selected: Option<V>, f: F) -> Self
    where
        V: Eq + Copy,
        F: FnOnce(V) -> Message,
    {
        Radio {
            is_selected: Some(value) == selected,
            on_click: f(value),
            label: label.into(),
            width: Length::Shrink,
            size: Self::DEFAULT_SIZE,
            spacing: Self::DEFAULT_SPACING,
            text_size: None,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::default(),
            wrapping: text::Wrapping::default(),
            font: None,
            on_status_change: None,
            class: Theme::default(),
            last_status: None,
        }
    }

    /// Sets the size of the [`Radio`] button.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the width of the [`Radio`] button.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the spacing between the [`Radio`] button and the text.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0;
        self
    }

    /// Sets the text size of the [`Radio`] button.
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Radio`] button.
    pub fn line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Radio`] button.
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the [`text::Wrapping`] strategy of the [`Radio`] button.
    pub fn wrapping(mut self, wrapping: text::Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Sets the text font of the [`Radio`] button.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the callback for status change notifications.
    ///
    /// The callback receives the new status name as a string
    /// (e.g. "active", "hovered", "focused").
    pub fn on_status_change(mut self, f: impl Fn(&str) -> Message + 'a) -> Self {
        self.on_status_change = Some(Box::new(f));
        self
    }

    /// Sets the style of the [`Radio`] button.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Radio`] button.
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

#[derive(Debug, Clone, Default)]
struct State<P: text::Paragraph> {
    is_focused: bool,
    focus_visible: bool,
    label: widget::text::State<P>,
}

impl<P: text::Paragraph> Focusable for State<P> {
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

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Radio<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::next_to_each_other(
            &limits.width(self.width),
            self.spacing,
            |_| layout::Node::new(Size::new(self.size, self.size)),
            |limits| {
                let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

                widget::text::layout(
                    &mut state.label,
                    renderer,
                    limits,
                    &self.label,
                    widget::text::Format {
                        width: self.width,
                        height: Length::Shrink,
                        line_height: self.line_height,
                        size: self.text_size,
                        font: self.font,
                        align_x: text::Alignment::Default,
                        align_y: alignment::Vertical::Top,
                        shaping: self.shaping,
                        wrapping: self.wrapping,
                        ellipsis: text::Ellipsis::default(),
                    },
                )
            },
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        operation.accessible(
            None,
            layout.bounds(),
            &Accessible {
                role: Role::RadioButton,
                label: Some(&self.label),
                selected: Some(self.is_selected),
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
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

                if cursor.is_over(layout.bounds()) {
                    state.is_focused = true;
                    state.focus_visible = false;

                    shell.publish(self.on_click.clone());
                    shell.capture_event();
                } else {
                    state.is_focused = false;
                    state.focus_visible = false;
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Space),
                ..
            }) => {
                let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

                if state.is_focused {
                    shell.publish(self.on_click.clone());
                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) => {
                let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
                if state.is_focused {
                    state.is_focused = false;
                    state.focus_visible = false;
                    shell.capture_event();
                }
            }
            _ => {}
        }

        let current_status = {
            let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
            let is_mouse_over = cursor.is_over(layout.bounds());
            let is_selected = self.is_selected;

            if state.focus_visible {
                Status::Focused { is_selected }
            } else if is_mouse_over {
                Status::Hovered { is_selected }
            } else {
                Status::Active { is_selected }
            }
        };

        let new_name = status_name(current_status);
        let old_name = self.last_status.map(status_name);
        if old_name != Some(new_name)
            && let Some(ref on_status_change) = self.on_status_change
        {
            shell.publish(on_status_change(new_name));
        }

        if (self.last_status.is_some_and(|s| s != current_status) || self.last_status.is_none())
            && !matches!(event, Event::Window(window::Event::RedrawRequested(_)))
        {
            shell.request_redraw();
        }
        self.last_status = Some(current_status);
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();

        let style = theme.style(
            &self.class,
            self.last_status.unwrap_or(Status::Active {
                is_selected: self.is_selected,
            }),
        );

        {
            let layout = children.next().unwrap();
            let bounds = layout.bounds();

            let size = bounds.width;
            let dot_size = size / 2.0;

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: (size / 2.0).into(),
                        width: style.border_width,
                        color: style.border_color,
                    },
                    shadow: style.shadow,
                    ..renderer::Quad::default()
                },
                style.background,
            );

            if self.is_selected {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + dot_size / 2.0,
                            y: bounds.y + dot_size / 2.0,
                            width: bounds.width - dot_size,
                            height: bounds.height - dot_size,
                        },
                        border: border::rounded(dot_size / 2.0),
                        ..renderer::Quad::default()
                    },
                    style.dot_color,
                );
            }
        }

        {
            let label_layout = children.next().unwrap();
            let state: &State<Renderer::Paragraph> = tree.state.downcast_ref();

            crate::text::draw(
                renderer,
                defaults,
                label_layout.bounds(),
                state.label.raw(),
                crate::text::Style {
                    color: style.text_color,
                },
                viewport,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Radio<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a + Catalog,
    Renderer: 'a + text::Renderer,
{
    fn from(radio: Radio<'a, Message, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(radio)
    }
}

/// The possible status of a [`Radio`] button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Radio`] button can be interacted with.
    Active {
        /// Indicates whether the [`Radio`] button is currently selected.
        is_selected: bool,
    },
    /// The [`Radio`] button is being hovered.
    Hovered {
        /// Indicates whether the [`Radio`] button is currently selected.
        is_selected: bool,
    },
    /// The [`Radio`] button has keyboard focus.
    Focused {
        /// Indicates whether the [`Radio`] button is currently selected.
        is_selected: bool,
    },
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Active { .. } => "active",
        Status::Hovered { .. } => "hovered",
        Status::Focused { .. } => "focused",
    }
}

/// The appearance of a radio button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the radio button.
    pub background: Background,
    /// The [`Color`] of the dot of the radio button.
    pub dot_color: Color,
    /// The border width of the radio button.
    pub border_width: f32,
    /// The border [`Color`] of the radio button.
    pub border_color: Color,
    /// The text [`Color`] of the radio button.
    pub text_color: Option<Color>,
    /// The [`Shadow`] of the radio button.
    pub shadow: Shadow,
}

/// The theme catalog of a [`Radio`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Radio`].
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

/// The default style of a [`Radio`] button.
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let active = Style {
        background: Color::TRANSPARENT.into(),
        dot_color: palette.primary.strong.color,
        border_width: 1.0,
        border_color: palette.primary.strong.color,
        text_color: None,
        shadow: Shadow::default(),
    };

    match status {
        Status::Active { .. } => active,
        Status::Hovered { .. } => Style {
            dot_color: palette.primary.strong.color,
            background: palette.primary.weak.color.into(),
            ..active
        },
        Status::Focused { .. } => {
            let page_bg = palette.background.base.color;
            Style {
                border_color: palette::focus_border_color(
                    Color::TRANSPARENT,
                    palette.primary.strong.color,
                    page_bg,
                ),
                border_width: 2.0,
                shadow: palette::focus_shadow(palette.primary.strong.color, page_bg),
                ..active
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::widget::operation::focusable::Focusable;

    type TestState = State<()>;

    #[test]
    fn focusable_trait() {
        let mut state = TestState::default();
        assert!(!state.is_focused());
        assert!(!state.focus_visible);
        state.focus();
        assert!(state.is_focused());
        assert!(state.focus_visible);
        state.unfocus();
        assert!(!state.is_focused());
        assert!(!state.focus_visible);
    }
}
