//! A group of radio buttons managed as a single tab stop.
//!
//! A [`RadioGroup`] renders N radio options vertically, manages focus
//! as one unit, and lets the user move between options with arrow keys
//! following the [WAI-ARIA radio group pattern](https://www.w3.org/WAI/ARIA/apg/patterns/radio/).
//!
//! Each option is drawn using the same circle-and-dot visual as
//! [`Radio`](crate::Radio), reusing [`radio::Catalog`] for styling.
use crate::core::alignment;
use crate::core::border::{self, Border};
use crate::core::keyboard;
use crate::core::keyboard::key;
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::operation::accessible::{Accessible, Role};
use crate::core::widget::operation::focusable::Focusable;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{Element, Event, Layout, Length, Pixels, Rectangle, Shell, Size, Widget};
use crate::radio;

/// A group of radio buttons that acts as a single tab stop with
/// arrow-key navigation.
///
/// Reuses [`radio::Catalog`] for visual styling so every option looks
/// identical to an individual [`Radio`](crate::Radio) button.
#[allow(missing_debug_implementations)]
pub struct RadioGroup<'a, V, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    V: Copy + Eq,
    Theme: radio::Catalog,
    Renderer: text::Renderer,
{
    options: Vec<(String, V)>,
    selected: Option<V>,
    on_select: Box<dyn Fn(V) -> Message + 'a>,
    size: f32,
    spacing: f32,
    option_spacing: f32,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
    shaping: text::Shaping,
    wrapping: text::Wrapping,
    font: Option<Renderer::Font>,
    class: Theme::Class<'a>,
    width: Length,
}

impl<'a, V, Message, Theme, Renderer> RadioGroup<'a, V, Message, Theme, Renderer>
where
    V: Copy + Eq,
    Theme: radio::Catalog,
    Renderer: text::Renderer,
{
    /// The default vertical spacing between options.
    pub const DEFAULT_OPTION_SPACING: f32 = 6.0;

    /// Creates a new [`RadioGroup`].
    ///
    /// It expects:
    ///   * an iterator of `(label, value)` pairs
    ///   * the currently selected value, if any
    ///   * a function that produces a `Message` when a value is selected
    pub fn new<F>(
        options: impl IntoIterator<Item = (impl Into<String>, V)>,
        selected: Option<V>,
        on_select: F,
    ) -> Self
    where
        F: Fn(V) -> Message + 'a,
    {
        RadioGroup {
            options: options
                .into_iter()
                .map(|(label, value)| (label.into(), value))
                .collect(),
            selected,
            on_select: Box::new(on_select),
            size: 16.0,
            spacing: 8.0,
            option_spacing: Self::DEFAULT_OPTION_SPACING,
            text_size: None,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::default(),
            wrapping: text::Wrapping::default(),
            font: None,
            class: Theme::default(),
            width: Length::Shrink,
        }
    }

    /// Sets the size of each radio circle.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the spacing between each radio circle and its label text.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0;
        self
    }

    /// Sets the vertical spacing between options.
    pub fn option_spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.option_spacing = spacing.into().0;
        self
    }

    /// Sets the text size of the option labels.
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text font of the option labels.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the width of the [`RadioGroup`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the style of the [`RadioGroup`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, radio::Status) -> radio::Style + 'a) -> Self
    where
        Theme::Class<'a>: From<radio::StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as radio::StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`RadioGroup`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

#[derive(Debug, Clone, Default)]
struct State<P: text::Paragraph> {
    active_index: usize,
    is_focused: bool,
    focus_visible: bool,
    labels: Vec<widget::text::State<P>>,
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

impl<V, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for RadioGroup<'_, V, Message, Theme, Renderer>
where
    V: Copy + Eq,
    Theme: radio::Catalog,
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
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        // Ensure we have the right number of label states.
        state
            .labels
            .resize_with(self.options.len(), Default::default);

        let limits = limits.width(self.width);
        let mut children = Vec::with_capacity(self.options.len());
        let mut total_height: f32 = 0.0;
        let mut max_width: f32 = 0.0;

        for (i, (label, _)) in self.options.iter().enumerate() {
            let node = layout::next_to_each_other(
                &limits,
                self.spacing,
                |_| layout::Node::new(Size::new(self.size, self.size)),
                |limits| {
                    widget::text::layout(
                        &mut state.labels[i],
                        renderer,
                        limits,
                        label,
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
            );

            let node_size = node.size();

            if i > 0 {
                total_height += self.option_spacing;
            }

            let node = node.move_to((0.0, total_height));
            total_height += node_size.height;
            max_width = max_width.max(node_size.width);

            children.push(node);
        }

        let size = limits.resolve(
            self.width,
            Length::Shrink,
            Size::new(max_width, total_height),
        );

        layout::Node::with_children(size, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let total = self.options.len();

        // Group container
        operation.accessible(
            None,
            layout.bounds(),
            &Accessible {
                role: Role::Group,
                ..Accessible::default()
            },
        );

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for (i, ((label, _), child_layout)) in
                self.options.iter().zip(layout.children()).enumerate()
            {
                operation.accessible(
                    None,
                    child_layout.bounds(),
                    &Accessible {
                        role: Role::RadioButton,
                        label: Some(label),
                        selected: Some(self.selected.is_some_and(|s| s == self.options[i].1)),
                        position_in_set: Some(i + 1),
                        size_of_set: Some(total),
                        ..Accessible::default()
                    },
                );

                // Text content for the label
                let mut label_children = child_layout.children();
                let _circle = label_children.next();
                if let Some(text_layout) = label_children.next() {
                    operation.text(None, text_layout.bounds(), label);
                }
            }
        });

        if total > 0 {
            operation.focusable(None, layout.bounds(), state);
        } else {
            state.unfocus();
        }
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
        let total = self.options.len();

        if total == 0 {
            return;
        }

        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        // Sync active_index with externally-changed selection.
        if let Some(selected) = self.selected
            && let Some(idx) = self.options.iter().position(|(_, v)| *v == selected)
        {
            state.active_index = idx;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                for (i, child_layout) in layout.children().enumerate() {
                    if cursor.is_over(child_layout.bounds()) {
                        state.active_index = i;
                        state.is_focused = true;
                        state.focus_visible = false;

                        shell.publish((self.on_select)(self.options[i].1));
                        shell.capture_event();
                        return;
                    }
                }

                // Click outside all options: clear focus.
                if cursor.is_over(layout.bounds()) {
                    // Inside group bounds but not on an option -- do nothing.
                } else {
                    state.is_focused = false;
                    state.focus_visible = false;
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::ArrowDown | key::Named::ArrowRight),
                ..
            }) if state.is_focused => {
                state.active_index = (state.active_index + 1) % total;
                shell.publish((self.on_select)(self.options[state.active_index].1));
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::ArrowUp | key::Named::ArrowLeft),
                ..
            }) if state.is_focused => {
                state.active_index = (state.active_index + total - 1) % total;
                shell.publish((self.on_select)(self.options[state.active_index].1));
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Space),
                ..
            }) if state.is_focused => {
                shell.publish((self.on_select)(self.options[state.active_index].1));
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) if state.is_focused => {
                state.is_focused = false;
                state.focus_visible = false;
                shell.capture_event();
            }
            _ => {}
        }

        // Redraw tracking: request redraw when status changes.
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            // Nothing to store; status is derived on the fly.
        } else {
            // A status change (focus, hover) may need a redraw.
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        for child_layout in layout.children() {
            if cursor.is_over(child_layout.bounds()) {
                return mouse::Interaction::Pointer;
            }
        }

        mouse::Interaction::default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        for (i, ((_label, value), option_layout)) in
            self.options.iter().zip(layout.children()).enumerate()
        {
            let is_selected = self.selected.is_some_and(|s| s == *value);
            let is_active = i == state.active_index;
            let is_mouse_over = cursor.is_over(option_layout.bounds());

            let status = if is_active && state.focus_visible {
                radio::Status::Focused { is_selected }
            } else if is_mouse_over {
                radio::Status::Hovered { is_selected }
            } else {
                radio::Status::Active { is_selected }
            };

            let style = theme.style(&self.class, status);

            let mut children = option_layout.children();

            // Draw the radio circle.
            {
                let circle_layout = children.next().unwrap();
                let bounds = circle_layout.bounds();
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
                        ..renderer::Quad::default()
                    },
                    style.background,
                );

                if is_selected {
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

            // Draw the label text.
            {
                let label_layout = children.next().unwrap();

                crate::text::draw(
                    renderer,
                    defaults,
                    label_layout.bounds(),
                    state.labels[i].raw(),
                    crate::text::Style {
                        color: style.text_color,
                    },
                    viewport,
                );
            }
        }
    }
}

impl<'a, V, Message, Theme, Renderer> From<RadioGroup<'a, V, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    V: 'a + Copy + Eq,
    Message: 'a,
    Theme: 'a + radio::Catalog,
    Renderer: 'a + text::Renderer,
{
    fn from(
        radio_group: RadioGroup<'a, V, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(radio_group)
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

    #[test]
    fn default_state_starts_at_zero() {
        let state = TestState::default();
        assert_eq!(state.active_index, 0);
        assert!(!state.is_focused());
    }
}
