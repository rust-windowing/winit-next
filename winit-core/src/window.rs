// TODO figure out how to do WindowId.

use std::any::Any;

pub use raw_window_handle::HasWindowHandle;
pub use raw_window_handle_05::HasRawWindowHandle as HasRawWindowHandle05;

use crate::dpi::{
    self, LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size,
};
use crate::monitor::MonitorId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u128);

pub enum WindowRole<'a> {
    Toplevel(&'a dyn Toplevel),
    Popup(&'a dyn Popup),
}

pub enum WindowRoleMut<'a> {
    Toplevel(&'a mut dyn Toplevel),
    Popup(&'a mut dyn Popup),
}

/// Common API for all rendering targets.
pub trait Surface: HasWindowHandle + HasRawWindowHandle05 {
    fn id(&self) -> WindowId;

    fn scale_factor(&self) -> f64;

    fn request_redraw(&mut self);

    fn inner_size(&self) -> PhysicalSize<u32>;

    /// Downcasts this surface to its specific type.
    fn role(&self) -> WindowRole;
    /// Downcasts this surface to its specific type. Returns a mutable reference.
    fn role_mut(&mut self) -> WindowRoleMut;
}

/// API for toplevel windows and dialogs.
pub trait Toplevel: Surface {
    /// Gets the current title of the window.
    fn title(&self) -> &str;

    fn set_title(&mut self, title: &str);

    fn theme(&self) -> Option<Theme>;

    fn set_theme(&mut self, theme: Option<Theme>);

    fn set_minimized(&mut self, minimize: bool);

    fn set_maximized(&mut self, maximized: bool);

    fn current_monitor(&self) -> Option<MonitorId>;

    fn primary_monitor(&self) -> Option<MonitorId>;
}

/// API for popups.
pub trait Popup: Surface {
    // fn set_position(&mut self, position: PhysicalPosition<u32>);
}

/// Attributes to use when creating a window.
#[derive(Debug, Clone)]
pub struct WindowAttributes {
    pub inner_size: Size,
    pub min_inner_size: Option<Size>,
    pub max_inner_size: Option<Size>,
    pub position: Option<Position>,
    pub resizable: bool,
    pub enabled_buttons: WindowButtons,
    pub title: String,
    pub maximized: bool,
    pub visible: bool,
    pub transparent: bool,
    pub blur: bool,
    pub decorations: bool,
    // pub window_icon: Option<Icon>,
    pub theme: Option<Theme>,
    pub resize_increments: Option<Size>,
    pub content_protected: bool,
    pub window_level: WindowLevel,
    pub active: bool,
    // pub cursor: Cursor,
    #[cfg(feature = "rwh_06")]
    pub(crate) parent_window: Option<SendSyncRawWindowHandle>,
    // pub fullscreen: Option<Fullscreen>,
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            inner_size: LogicalSize::new(800, 600).into(),
            enabled_buttons: WindowButtons::all(),
            title: String::from("Winit window"),
            content_protected: false,
            resize_increments: None,
            max_inner_size: None,
            min_inner_size: None,
            transparent: true,
            decorations: true,
            maximized: false,
            resizable: true,
            position: None,
            visible: false,
            active: true,
            theme: None,
            blur: false,
            window_level: Default::default(),
        }
    }
}

impl WindowAttributes {
    #[inline]
    pub fn inner_size(&self) -> Size {
        self.inner_size
    }

    /// Requests the window to be of specific dimensions.
    #[inline]
    pub fn with_inner_size<S: Into<Size>>(mut self, inner_size: S) -> Self {
        self.inner_size = inner_size.into();
        self
    }

    #[inline]
    pub fn min_inner_size(&self) -> Option<Size> {
        self.min_inner_size
    }

    /// Sets the minimum dimensions a window can have.
    ///
    /// If this is not set, the window will have no minimum dimensions (aside
    /// from reserved).
    #[inline]
    pub fn with_min_inner_size<S: Into<Size>>(mut self, min_size: S) -> Self {
        self.min_inner_size = Some(min_size.into());
        self
    }

    #[inline]
    pub fn max_inner_size(&self) -> Option<Size> {
        self.max_inner_size
    }

    /// Sets the maximum dimensions a window can have.
    ///
    /// If this is not set, the window will have no maximum dimensions (aside
    /// from reserved).
    #[inline]
    pub fn with_max_inner_size<S: Into<Size>>(mut self, max_size: S) -> Self {
        self.max_inner_size = Some(max_size.into());
        self
    }

    #[inline]
    pub fn position(&self) -> Option<Position> {
        self.position
    }

    /// Sets a desired initial position for the window.
    ///
    /// If this is not set, some platform-specific position will be chosen.
    #[inline]
    pub fn with_position<P: Into<Position>>(mut self, position: P) -> Self {
        self.position = Some(position.into());
        self
    }

    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[inline]
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = title.into();
        self
    }

    #[inline]
    pub fn maximized(&self) -> bool {
        self.maximized
    }

    /// Request that the window is maximized upon creation.
    ///
    /// The default is `false`.
    ///
    /// See [`Window::set_maximized`] for details.
    #[inline]
    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    #[inline]
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Sets whether the window will be initially visible or hidden.
    ///
    /// The default is to show the window.
    ///
    /// See [`Window::set_visible`] for details.
    #[inline]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    #[inline]
    pub fn resizable(&self) -> bool {
        self.resizable
    }

    /// Sets whether the window is resizable or not.
    ///
    /// The default is `true`.
    ///
    /// See [`Window::set_resizable`] for details.
    #[inline]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    #[inline]
    pub fn transparent(&self) -> bool {
        self.transparent
    }

    /// Sets whether the background of the window should be transparent.
    ///
    /// If this is `true`, writing colors with alpha values different than
    /// `1.0` will produce a transparent window. On some platforms this
    /// is more of a hint for the system and you'd still have the alpha
    /// buffer. To control it see [`Window::set_transparent`].
    ///
    /// The default is `false`.
    #[inline]
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    #[inline]
    pub fn blur(&self) -> bool {
        self.blur
    }

    /// Sets whether the background of the window should be blurred by the
    /// system.
    ///
    /// The default is `false`.
    ///
    /// See [`Window::set_blur`] for details.
    #[inline]
    pub fn with_blur(mut self, blur: bool) -> Self {
        self.blur = blur;
        self
    }

    #[inline]
    pub fn decorations(&self) -> bool {
        self.decorations
    }

    /// Sets whether the window should have a border, a title bar, etc.
    ///
    /// The default is `true`.
    ///
    /// See [`Window::set_decorations`] for details.
    #[inline]
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    #[inline]
    pub fn window_level(&self) -> WindowLevel {
        self.window_level
    }

    /// Sets the window level.
    ///
    /// This is just a hint to the OS, and the system could ignore it.
    ///
    /// The default is [`WindowLevel::Normal`].
    ///
    /// See [`WindowLevel`] for details.
    #[inline]
    pub fn with_window_level(mut self, level: WindowLevel) -> Self {
        self.window_level = level;
        self
    }

    #[inline]
    pub fn theme(&self) -> Option<Theme> {
        self.theme
    }

    /// Sets a specific theme for the window.
    ///
    /// If `None` is provided, the window will use the system theme.
    ///
    /// The default is `None`.
    #[inline]
    pub fn with_theme(mut self, theme: Option<Theme>) -> Self {
        self.theme = theme;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AnchorDirection {
    Center = 0,
    North = 1,
    Northeast = 2,
    East = 3,
    Southeast = 4,
    South = 5,
    Southwest = 6,
    West = 7,
    Northwest = 8,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct AnchorHints: u32 {
        const NONE = 0;
        const FLIP_X = 1 << 0;
        const FLIP_Y = 1 << 1;
        const SLIDE_X = 1 << 2;
        const SLIDE_Y = 1 << 3;
        const RESIZE_X = 1 << 4;
        const RESIZE_Y = 1 << 5;
    }
}

#[derive(Debug, Clone)]
pub struct PopupAttributes {
    pub inner_size: Size,
    pub anchor_rect: (Position, Size),
    pub rect_anchor: AnchorDirection,
    pub surface_anchor: AnchorDirection,
    pub offset: Position,
    pub anchor_hints: AnchorHints,
}

impl Default for PopupAttributes {
    fn default() -> Self {
        Self {
            inner_size: LogicalSize::new(100.0, 75.0).into(),
            anchor_rect: (
                LogicalPosition::new(0.0, 0.0).into(),
                LogicalSize::new(100.0, 100.0).into(),
            ),
            rect_anchor: AnchorDirection::Center,
            surface_anchor: AnchorDirection::Center,
            offset: LogicalPosition::new(0.0, 0.0).into(),
            anchor_hints: AnchorHints::NONE,
        }
    }
}

impl PopupAttributes {
    pub fn inner_size(&self) -> Size {
        self.inner_size
    }

    pub fn with_inner_size<S: Into<Size>>(mut self, size: S) -> Self {
        self.inner_size = size.into();
        self
    }

    pub fn anchor_rect(&self) -> (Position, Size) {
        self.anchor_rect
    }

    pub fn with_anchor_rect<P: Into<Position>, S: Into<Size>>(
        mut self,
        top_left: P,
        size: S,
    ) -> Self {
        self.anchor_rect = (top_left.into(), size.into());
        self
    }

    pub fn rect_anchor(&self) -> AnchorDirection {
        self.rect_anchor
    }

    pub fn with_rect_anchor(mut self, direction: AnchorDirection) -> Self {
        self.rect_anchor = direction;
        self
    }

    pub fn surface_anchor(&self) -> AnchorDirection {
        self.surface_anchor
    }

    pub fn with_surface_anchor(mut self, direction: AnchorDirection) -> Self {
        self.surface_anchor = direction;
        self
    }

    pub fn offset(&self) -> Position {
        self.offset
    }

    pub fn with_offset<P: Into<Position>>(mut self, offset: P) -> Self {
        self.offset = offset.into();
        self
    }

    pub fn anchor_hints(&self) -> AnchorHints {
        self.anchor_hints
    }

    pub fn with_anchor_hints(mut self, hints: AnchorHints) -> Self {
        self.anchor_hints = hints;
        self
    }
}

/// A window level groups windows with respect to their z-position.
///
/// The relative ordering between windows in different window levels is fixed.
/// The z-order of a window within the same window level may change dynamically
/// on user interaction.
///
/// ## Platform-specific
///
/// - **iOS / Android / Web / Wayland:** Unsupported.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum WindowLevel {
    /// The window will always be below normal windows.
    ///
    /// This is useful for a widget-based app.
    AlwaysOnBottom,

    /// The default.
    #[default]
    Normal,

    /// The window will always be on top of normal windows.
    AlwaysOnTop,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WindowButtons: u32 {
        const CLOSE  = 1 << 0;
        const MINIMIZE  = 1 << 1;
        const MAXIMIZE  = 1 << 2;
    }
}

/// The theme variant to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    /// Use the light variant.
    Light,

    /// Use the dark variant.
    Dark,
}
