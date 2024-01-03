use std::num::NonZeroU32;
use std::sync::Arc;

use raw_window_handle::{HandleError, HasWindowHandle, WaylandWindowHandle, WindowHandle};
use raw_window_handle_05::HasRawWindowHandle as HasRawWindowHandle05;

use sctk::compositor::{CompositorHandler, CompositorState, Region, SurfaceData};
use sctk::reexports::client::Proxy;
use sctk::reexports::csd_frame::{
    DecorationsFrame, FrameAction, FrameClick, ResizeEdge, WindowState as XdgWindowState,
};
use sctk::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use sctk::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3::ZwpTextInputV3;
use sctk::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use sctk::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge as XdgResizeEdge;
use sctk::shell::xdg::window::{
    DecorationMode, Window as XdgWindow, WindowConfigure, WindowDecorations, WindowHandler,
};
use sctk::shell::xdg::XdgSurface;
use sctk::shell::WaylandSurface;

use wayland_client::{Connection, QueueHandle};
use winit_core::dpi::{LogicalSize, PhysicalSize, Size};
use winit_core::monitor::MonitorId;
use winit_core::window::{Theme, Window as CoreWindow, WindowAttributes, WindowId};

use crate::event_loop::RuntimeState;
use crate::logical_to_physical_rounded;
use crate::monitor::Monitor;
use crate::state::WinitState;

// Minimum window inner size.
const MIN_WINDOW_SIZE: LogicalSize<u32> = LogicalSize::new(2, 1);

#[cfg(feature = "sctk-adwaita")]
type WinitFrame = sctk_adwaita::AdwaitaFrame<RuntimeState>;
#[cfg(not(feature = "sctk-adwaita"))]
type WinitFrame = sctk::shell::xdg::fallback_frame::FallbackFrame<RuntimeState>;

pub struct Window {
    /// The last received configure.
    pub last_configure: Option<WindowConfigure>,

    pub viewport: Option<WpViewport>,
    fractional_scale: Option<WpFractionalScaleV1>,

    /// The window frame, which is created from the configure request.
    frame: Option<WinitFrame>,

    /// The scale factor of the window.
    pub scale_factor: f64,

    /// The latest requested window title.
    title: String,

    /// Whether the window has focus.
    has_focus: bool,

    /// Whether the frame is resizable.
    resizable: bool,

    /// The inner size of the window, as in without client side decorations.
    size: LogicalSize<u32>,

    /// The size of the window when no states were applied to it. The primary
    /// use for it is to fallback to original window size, before it was
    /// maximized, if the compositor sends `None` for the new size in the
    /// configure.
    stateless_size: LogicalSize<u32>,

    /// Initial window size provided by the user. Removed on the first
    /// configure.
    initial_size: Option<Size>,

    compositor: Arc<CompositorState>,

    /// Theme varaint.
    theme: Option<Theme>,

    /// Min size.
    min_inner_size: LogicalSize<u32>,
    max_inner_size: Option<LogicalSize<u32>>,

    /// Whether we should decorate the frame.
    decorate: bool,

    /// Whether the window is transparent.
    transparent: bool,

    /// Whether the CSD fail to create, so we don't try to create them on each
    /// iteration.
    csd_fails: bool,

    pub redraw: bool,

    // Note, should be the last since it drops the surface.
    pub window: XdgWindow,
}

impl Window {
    pub fn new(winit: &mut WinitState, attributes: &WindowAttributes) -> Self {
        let compositor = winit.compositor.clone();
        let surface = compositor.create_surface(&winit.queue_handle);

        // We prefer server side decorations, however to not have decorations we ask for
        // client side decorations instead.
        let decorate = if attributes.decorations() {
            WindowDecorations::RequestServer
        } else {
            WindowDecorations::RequestClient
        };

        let window = winit.xdg_shell.create_window(surface.clone(), decorate, &winit.queue_handle);

        let viewport = None;
        let fractional_scale = None;

        let size = attributes.inner_size();

        // Create the window with some defaults.
        let mut window = Self {
            decorate: attributes.decorations(),
            min_inner_size: MIN_WINDOW_SIZE,
            stateless_size: size.to_logical(1.),
            size: size.to_logical(1.),
            initial_size: Some(size),
            max_inner_size: None,
            last_configure: None,
            transparent: true,
            fractional_scale,
            scale_factor: 1.,
            has_focus: false,
            csd_fails: false,
            resizable: true,
            redraw: false,
            frame: None,
            theme: None,
            compositor,
            viewport,
            window,
            title: Default::default(),
        };

        window.set_title(attributes.title());

        // Set transparency hint.
        window.set_transparent(attributes.transparent);

        window.set_min_inner_size(attributes.min_inner_size());
        window.set_max_inner_size(attributes.max_inner_size());

        window.set_resizable(attributes.resizable());

        // window.set_

        if attributes.maximized() {
            window.set_maximized(true);
        }

        // TODO: platform attributes.

        // NOTE: initial commit for the window.
        window.window.commit();

        window
    }

    /// Set the resizable state on the window.
    #[inline]
    pub fn set_resizable(&mut self, resizable: bool) {
        if self.resizable == resizable {
            return;
        }

        self.resizable = resizable;

        if resizable {
            // Restore min/max sizes of the window.
            self.reload_min_max_hints();
        } else {
            self.set_min_inner_size(Some(self.size.into()));
            self.set_max_inner_size(Some(self.size.into()));
        }

        // Reload the state on the frame as well.
        if let Some(frame) = self.frame.as_mut() {
            frame.set_resizable(resizable);
        }
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
        self.reload_transparency_hint();
    }

    pub fn set_min_inner_size(&mut self, size: Option<Size>) {
        let mut size =
            size.map(|size| size.to_logical(self.scale_factor)).unwrap_or(MIN_WINDOW_SIZE);
        size.height = size.width.max(MIN_WINDOW_SIZE.width);
        size.width = size.width.max(MIN_WINDOW_SIZE.width);
        // TODO borders
        self.window.set_min_size(Some(size.into()));
        self.min_inner_size = size;
    }

    pub fn set_max_inner_size(&mut self, size: Option<Size>) {
        let size = size.map(|size| size.to_logical(self.scale_factor));
        self.window.set_max_size(size.map(Into::into));
        self.max_inner_size = size;
    }

    pub(crate) fn resize(&mut self, new_size: LogicalSize<u32>) {
        self.size = new_size;

        // Update the stateless size.
        if Some(true) == self.last_configure.as_ref().map(Self::is_stateless) {
            self.stateless_size = self.size;
        }

        // Update the inner frame.
        let ((x, y), outer_size) = if let Some(frame) = self.frame.as_mut() {
            // Resize only visible frame.
            if !frame.is_hidden() {
                frame.resize(
                    NonZeroU32::new(self.size.width).unwrap(),
                    NonZeroU32::new(self.size.height).unwrap(),
                );
            }

            (frame.location(), frame.add_borders(self.size.width, self.size.height).into())
        } else {
            ((0, 0), self.size)
        };

        // Reload the hint.
        self.reload_transparency_hint();

        // Set the window geometry.
        self.window.xdg_surface().set_window_geometry(
            x,
            y,
            outer_size.width as i32,
            outer_size.height as i32,
        );

        // Update the target viewport, this is used if and only if fractional scaling is
        // in use.
        if let Some(viewport) = self.viewport.as_ref() {
            // Set inner size without the borders.
            viewport.set_destination(self.size.width as _, self.size.height as _);
        }
    }

    #[inline]
    pub(crate) fn is_stateless(configure: &WindowConfigure) -> bool {
        !(configure.is_maximized() || configure.is_fullscreen() || configure.is_tiled())
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;

        // NOTE: When fractional scaling is not used update the buffer scale.
        if self.fractional_scale.is_none() {
            let _ = self.window.set_buffer_scale(self.scale_factor as _);
        }

        if let Some(frame) = self.frame.as_mut() {
            frame.set_scaling_factor(scale_factor);
        }
    }

    /// Reissue the transparency hint to the compositor.
    pub(crate) fn reload_transparency_hint(&self) {
        let surface = self.window.wl_surface();

        if self.transparent {
            surface.set_opaque_region(None);
        } else if let Ok(region) = Region::new(&*self.compositor) {
            region.add(0, 0, i32::MAX, i32::MAX);
            surface.set_opaque_region(Some(region.wl_region()));
        } else {
            // warn!("Failed to mark window opaque.");
        }
    }

    /// Reload the hints for minimum and maximum sizes.
    pub(crate) fn reload_min_max_hints(&mut self) {
        self.set_min_inner_size(Some(self.min_inner_size.into()));
        self.set_max_inner_size(self.max_inner_size.map(Into::into));
    }

    pub(crate) fn configured(&self) -> bool {
        self.last_configure.is_some()
    }
}

impl CoreWindow for Window {
    fn id(&self) -> WindowId {
        crate::make_wid(&self.window.wl_surface())
    }

    fn request_redraw(&mut self) {
        self.redraw = true;
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn theme(&self) -> Option<Theme> {
        self.theme
    }

    fn set_theme(&mut self, theme: Option<Theme>) {
        self.theme = theme;
        #[cfg(feature = "sctk-adwaita")]
        if let Some(frame) = self.frame.as_mut() {
            frame.set_config(into_sctk_adwaita_config(theme))
        }
    }

    fn set_title(&mut self, title: &str) {
        let mut title = title.to_owned();
        // Truncate the title to at most 1024 bytes, so that it does not blow up the
        // protocol messages
        if title.len() > 1024 {
            let mut new_len = 1024;
            while !title.is_char_boundary(new_len) {
                new_len -= 1;
            }
            title.truncate(new_len);
        }

        // Update the CSD title.
        if let Some(frame) = self.frame.as_mut() {
            frame.set_title(&title);
        }

        self.window.set_title(&title);
        self.title = title;
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn inner_size(&self) -> PhysicalSize<u32> {
        crate::logical_to_physical_rounded(self.size, self.scale_factor)
    }

    fn set_minimized(&mut self, minimize: bool) {
        if minimize {
            self.window.set_minimized();
        }
    }

    fn set_maximized(&mut self, maximized: bool) {
        if maximized {
            self.window.set_maximized();
        } else {
            self.window.unset_maximized();
        }
    }

    fn current_monitor(&self) -> Option<MonitorId> {
        let data = self.window.wl_surface().data::<SurfaceData>()?;
        data.outputs().next().as_ref().map(crate::make_mid)
    }

    fn primary_monitor(&self) -> Option<MonitorId> {
        None
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let ptr = self.window.wl_surface().id().as_ptr();
        let handle = WaylandWindowHandle::new({
            std::ptr::NonNull::new(ptr as *mut _).expect("wl_surface will never be null")
        });

        unsafe { Ok(WindowHandle::borrow_raw(handle.into())) }
    }
}

impl WindowHandler for RuntimeState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &XdgWindow) {
        let window_id = crate::make_wid(window.wl_surface());
        let user_state = self.user.as_mut().unwrap();
        if user_state.close_requested(&mut self.winit, window_id) {
            // Instantly drop the window.
            drop(self.winit.windows.remove(&window_id));
            user_state.destroyed(&mut self.winit, window_id);
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        queue_handle: &QueueHandle<Self>,
        window: &XdgWindow,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let winit = &mut self.winit;
        let window_id = crate::make_wid(window.wl_surface());
        let window = match winit.windows.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        let scale_factor = window.scale_factor;

        // NOTE: when using fractional scaling or wl_compositor@v6 the scaling
        // should be delivered before the first configure, thus apply it to
        // properly scale the physical sizes provided by the users.
        if let Some(initial_size) = window.initial_size.take() {
            window.size = initial_size.to_logical(scale_factor);
            window.stateless_size = window.size;
        }

        if let Some(subcompositor) = winit.subcompositor.as_ref().filter(|_| {
            configure.decoration_mode == DecorationMode::Client
                && window.frame.is_none()
                && !window.csd_fails
        }) {
            match WinitFrame::new(
                &window.window,
                &winit.shm,
                #[cfg(feature = "sctk-adwaita")]
                winit.compositor.clone(),
                subcompositor.clone(),
                queue_handle.clone(),
                #[cfg(feature = "sctk-adwaita")]
                into_sctk_adwaita_config(window.theme),
            ) {
                Ok(mut frame) => {
                    frame.set_title(&window.title);
                    frame.set_scaling_factor(scale_factor);
                    // Hide the frame if we were asked to not decorate.
                    frame.set_hidden(!window.decorate);
                    window.frame = Some(frame);
                },
                Err(err) => {
                    // warn!("Failed to create client side decorations frame: {err}");
                    window.csd_fails = true;
                },
            }
        } else if configure.decoration_mode == DecorationMode::Server {
            // Drop the frame for server side decorations to save resources.
            window.frame = None;
        }

        let (new_size, constrain): (LogicalSize<u32>, bool) = match configure.new_size {
            (Some(width), Some(height)) => ((width.get(), height.get()).into(), false),
            _ => (window.size, true),
        };

        let user = self.user.as_mut().unwrap();
        let initial_configue = window.last_configure.is_none();
        window.last_configure = Some(configure);

        window.resize(new_size);

        // NOTE: we consider window as created when its initial configure arrives, until
        // then it's considered as not created and attempt to get it will result in
        // error.
        if initial_configue {
            user.created(winit, window_id);
            user.scale_factor_changed(winit, window_id, scale_factor);
        }

        user.resized(winit, window_id, logical_to_physical_rounded(new_size, scale_factor));

        if initial_configue {
            user.redraw_requested(winit, window_id);
        }
    }
}

unsafe impl HasRawWindowHandle05 for Window {
    fn raw_window_handle(&self) -> raw_window_handle_05::RawWindowHandle {
        let mut window_handle = raw_window_handle_05::WaylandWindowHandle::empty();
        window_handle.surface = self.window.wl_surface().id().as_ptr() as *mut _;
        raw_window_handle_05::RawWindowHandle::Wayland(window_handle)
    }
}

#[cfg(feature = "sctk-adwaita")]
fn into_sctk_adwaita_config(theme: Option<Theme>) -> sctk_adwaita::FrameConfig {
    match theme {
        Some(Theme::Light) => sctk_adwaita::FrameConfig::light(),
        Some(Theme::Dark) => sctk_adwaita::FrameConfig::dark(),
        None => sctk_adwaita::FrameConfig::auto(),
    }
}
