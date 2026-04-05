//! Operate on widgets that expose accessibility metadata.
//!
//! Unlike the sibling [`focusable`], [`scrollable`], and [`text_input`]
//! modules, this module does not define a trait for widgets to implement.
//! Instead, widgets construct an [`Accessible`] value and pass it to
//! [`Operation::accessible`] during their [`operate`] method.
//!
//! [`focusable`]: super::focusable
//! [`scrollable`]: super::scrollable
//! [`text_input`]: super::text_input
//! [`Operation::accessible`]: super::Operation::accessible
//! [`operate`]: crate::widget::Widget::operate

use crate::widget;

/// The role a widget plays in the accessibility tree.
///
/// Used by assistive technology to convey the purpose and interaction
/// model of a widget to the user. Defaults to [`Role::Group`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Role {
    /// An alert message.
    Alert,
    /// A dialog conveying an alert.
    AlertDialog,
    /// A push button.
    Button,
    /// A canvas element for custom drawing.
    Canvas,
    /// A check box.
    CheckBox,
    /// A combo box / drop-down list.
    ComboBox,
    /// A dialog or modal window.
    Dialog,
    /// A document-like content area.
    Document,
    /// A generic container grouping related widgets.
    #[default]
    Group,
    /// A container with no special semantic role.
    ///
    /// Unlike [`Group`](Self::Group), which implies a grouping
    /// relationship between its children, `GenericContainer` is
    /// semantically neutral. Use it for layout wrappers that carry
    /// no meaning for assistive technology.
    GenericContainer,
    /// A heading element (used with levels 1--6).
    Heading,
    /// A raster or vector image.
    Image,
    /// A label for another widget.
    Label,
    /// A hyperlink.
    Link,
    /// A list container.
    List,
    /// An item within a list.
    ListItem,
    /// A menu container.
    Menu,
    /// A menu bar container.
    MenuBar,
    /// An item within a menu.
    MenuItem,
    /// A meter or gauge.
    Meter,
    /// A multiline text input field.
    MultilineTextInput,
    /// A navigation landmark.
    Navigation,
    /// A progress indicator.
    ProgressIndicator,
    /// A radio button.
    RadioButton,
    /// A container for a set of radio buttons.
    RadioGroup,
    /// A generic landmark region.
    Region,
    /// A scrollbar control.
    ScrollBar,
    /// A scrollable area.
    ScrollView,
    /// A search landmark.
    Search,
    /// A visual separator between sections.
    Separator,
    /// A slider control.
    Slider,
    /// Non-interactive text content.
    StaticText,
    /// A status message area.
    Status,
    /// A toggle switch.
    Switch,
    /// A single tab within a tab list.
    Tab,
    /// A container of tabs.
    TabList,
    /// A panel associated with a tab.
    TabPanel,
    /// A data table.
    Table,
    /// A row within a data table.
    Row,
    /// A cell within a table row.
    Cell,
    /// A column header cell within a table.
    ColumnHeader,
    /// A text input field.
    TextInput,
    /// A toolbar container.
    Toolbar,
    /// A tooltip popup.
    Tooltip,
    /// A tree view.
    Tree,
    /// An item within a tree view.
    TreeItem,
    /// A window or pane.
    Window,
}

/// The current value of a widget, exposed to assistive technology.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    /// A textual value (e.g. text input content).
    Text(&'a str),
    /// A numeric value with its valid range.
    Numeric {
        /// The current value.
        current: f64,
        /// The minimum value.
        min: f64,
        /// The maximum value.
        max: f64,
        /// The step increment, if any.
        step: Option<f64>,
    },
}

/// How urgently assistive technology should announce changes to a
/// widget's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Live {
    /// Changes are announced at the next graceful opportunity.
    Polite,
    /// Changes are announced immediately, interrupting the current
    /// speech.
    Assertive,
}

/// The orientation of a widget (e.g. a slider or toolbar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Horizontal layout (default for most widgets).
    Horizontal,
    /// Vertical layout.
    Vertical,
}

/// The type of popup triggered by a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HasPopup {
    /// A listbox popup (used by combobox and pick list).
    Listbox,
    /// A menu popup (used by menu buttons).
    Menu,
    /// A dialog popup (used by modal triggers).
    Dialog,
    /// A tree popup (used by tree-view triggers).
    Tree,
    /// A grid popup (used by grid-view triggers).
    Grid,
}

/// Accessibility metadata for a single widget.
///
/// Passed to [`Operation::accessible`] by widgets that wish to
/// participate in the accessibility tree. All fields beyond [`role`]
/// are optional; widgets should only populate the fields that apply.
///
/// [`role`]: Accessible::role
/// [`Operation::accessible`]: super::Operation::accessible
#[derive(Debug, Clone, Default)]
pub struct Accessible<'a> {
    /// The semantic role of the widget.
    pub role: Role,
    /// A human-readable name for the widget (e.g. button label).
    pub label: Option<&'a str>,
    /// A longer human-readable description (e.g. tooltip text).
    pub description: Option<&'a str>,
    /// The current value, if the widget carries one.
    pub value: Option<Value<'a>>,
    /// Whether the widget is disabled.
    pub disabled: bool,
    /// The toggle state, for widgets like check boxes and switches.
    pub toggled: Option<bool>,
    /// The selection state, for widgets like radio buttons and list
    /// items.
    pub selected: Option<bool>,
    /// Whether a collapsible section is expanded.
    pub expanded: Option<bool>,
    /// The live-region setting, for widgets whose content changes
    /// should be announced by assistive technology.
    pub live: Option<Live>,
    /// The heading level (1--6), for widgets with [`Role::Heading`].
    pub level: Option<usize>,
    /// Whether the widget is required (e.g. a required form field).
    pub required: bool,
    /// The widget's orientation, for sliders and toolbars.
    pub orientation: Option<Orientation>,
    /// Another widget that provides this widget's label.
    ///
    /// Use this instead of [`label`](Self::label) when the label
    /// comes from a separate widget in the tree.
    pub labelled_by: Option<&'a widget::Id>,
    /// Another widget that provides this widget's description.
    ///
    /// Use this instead of [`description`](Self::description) when
    /// the description comes from a separate widget in the tree.
    pub described_by: Option<&'a widget::Id>,
    /// Position of this item in a set (1-based). Used for list items,
    /// radio buttons, and tab panels.
    pub position_in_set: Option<usize>,
    /// Total number of items in the set containing this item.
    pub size_of_set: Option<usize>,
    /// The currently active child in a composite widget (e.g. the
    /// highlighted option in a combobox popup).
    pub active_descendant: Option<&'a widget::Id>,
    /// The type of popup this widget triggers when activated.
    pub has_popup: Option<HasPopup>,
    /// The IDs of all radio buttons in this radio group.
    ///
    /// Set on each radio button in a group so assistive technology
    /// knows which buttons belong together.
    pub radio_group: Option<&'a [widget::Id]>,
    /// Whether the widget's value is invalid (form validation).
    pub invalid: bool,
    /// A widget that describes why the value is invalid.
    ///
    /// Points to a separate widget containing the error text. The
    /// screen reader announces the error when the user navigates to
    /// the invalid field.
    pub error_message: Option<&'a widget::Id>,
    /// Whether the widget is read-only (viewable but not editable).
    ///
    /// Distinct from disabled: read-only widgets are navigable and
    /// their values can be selected/copied, but not changed.
    pub read_only: bool,
    /// Whether the widget is busy (loading or processing).
    pub busy: bool,
    /// Whether the widget is hidden from assistive technology.
    ///
    /// Used to hide background content when a modal dialog is open.
    pub hidden: bool,
    /// Whether this dialog is modal.
    ///
    /// When set on a Dialog node, assistive technology restricts
    /// interaction to the dialog's content.
    pub modal: bool,
    /// The keyboard mnemonic for this widget (Alt+letter).
    ///
    /// When the user presses Alt plus this character, the widget is
    /// focused and activated.
    pub mnemonic: Option<char>,
}
