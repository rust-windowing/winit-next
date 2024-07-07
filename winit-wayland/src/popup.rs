use std::{marker::PhantomData, ops::Deref, ptr::NonNull, sync::Arc};

use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, WaylandWindowHandle, WindowHandle,
};
use raw_window_handle_05::HasRawWindowHandle as HasRawWindowHandle05;
use sctk::{
    compositor::CompositorState,
    shell::{
        self,
        xdg::{
            popup::{Popup as XdgPopup, PopupConfigure, PopupHandler},
            XdgPositioner, XdgSurface,
        },
    },
};
use wayland_client::{Dispatch, Proxy};
use wayland_protocols::{
    wp::{
        fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
        viewporter::client::wp_viewport::WpViewport,
    },
    xdg::shell::client::xdg_positioner,
};
use winit_core::{
    application::Application,
    dpi::{LogicalSize, PhysicalSize, Size},
    window::{
        AnchorDirection, AnchorHints, Popup as WinitPopup, PopupAttributes,
        Surface as WinitSurface, Theme, WindowAttributes, WindowId, WindowRole, WindowRoleMut,
    },
};

use crate::{event_loop::RuntimeState, state::WinitState, window::Window};

pub struct Popup<T: Application + 'static> {
    /// The last received configure.
    pub last_configure: Option<PopupConfigure>,

    pub viewport: Option<WpViewport>,
    fractional_scale: Option<WpFractionalScaleV1>,

    /// The scale factor of the window.
    pub scale_factor: f64,

    /// The inner size of the window, as in without client side decorations.
    size: LogicalSize<u32>,

    /// Initial window size provided by the user. Removed on the first
    /// configure.
    initial_size: Option<Size>,

    compositor: Arc<CompositorState>,

    /// Whether the window is transparent.
    transparent: bool,

    pub redraw: bool,

    // Note, should be the last since it drops the surface.
    pub popup: XdgPopup,

    _phantom_tparam_do_not_use: PhantomData<T>,
}

impl<T: Application + 'static> Popup<T> {
    pub fn new(
        winit: &mut WinitState<T>,
        parent: WindowId,
        attributes: &PopupAttributes,
    ) -> Result<Self, ()> {
        let parent = winit.windows.get(&parent).ok_or(())?;

        let compositor = winit.compositor.clone();
        let surface = compositor.create_surface(&winit.queue_handle);

        let positioner = {
            let positioner = XdgPositioner::new(&winit.xdg_shell).unwrap();

            let anchor_top_left = attributes.anchor_rect().0.to_physical::<i32>(1.0);
            let anchor_size = attributes.anchor_rect().1.to_physical::<i32>(1.0);
            positioner.set_anchor_rect(
                anchor_top_left.x,
                anchor_top_left.y,
                anchor_size.width,
                anchor_size.height,
            );

            let inner_size = attributes.inner_size().to_physical::<i32>(1.0);
            positioner.set_size(inner_size.width, inner_size.height);

            positioner.set_gravity(to_xdg_positioner_gravity(attributes.surface_anchor()));
            positioner.set_anchor(to_xdg_positioner_anchor(attributes.rect_anchor()));

            positioner
                .set_constraint_adjustment(to_xdg_constraints(attributes.anchor_hints()).bits());

            positioner
        };

        let popup = XdgPopup::new(
            parent.xdg_surface(),
            &positioner.deref(),
            &winit.queue_handle,
            &*winit.compositor,
            &winit.xdg_shell,
        )
        .unwrap();
        let size = attributes.inner_size();

        Ok(Self {
            last_configure: None,
            viewport: None,
            fractional_scale: None,
            scale_factor: 1.0,
            size: size.to_logical(1.0),
            initial_size: Some(size),
            compositor,
            transparent: true,
            redraw: false,
            popup,
            _phantom_tparam_do_not_use: PhantomData,
        })
    }

    pub fn configured(&self) -> bool {
        self.last_configure.is_some()
    }
}

impl<T: Application + 'static> WinitSurface for Popup<T> {
    fn id(&self) -> WindowId {
        crate::make_wid(self.popup.wl_surface())
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn request_redraw(&mut self) {
        self.redraw = true;
    }

    fn inner_size(&self) -> winit_core::dpi::PhysicalSize<u32> {
        crate::logical_to_physical_rounded(self.size, self.scale_factor)
    }

    fn role(&self) -> WindowRole<'_> {
        WindowRole::Popup(self)
    }

    fn role_mut(&mut self) -> WindowRoleMut {
        WindowRoleMut::Popup(self)
    }
}

impl<T: Application + 'static> WinitPopup for Popup<T> {}

impl<T: Application + 'static> HasWindowHandle for Popup<T> {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let ptr = self.popup.wl_surface().id().as_ptr();
        let handle = WaylandWindowHandle::new({
            NonNull::new(ptr as *mut _).expect("wl_surface should never be null")
        });

        unsafe { Ok(WindowHandle::borrow_raw(handle.into())) }
    }
}

unsafe impl<T: Application + 'static> HasRawWindowHandle05 for Popup<T> {
    fn raw_window_handle(&self) -> raw_window_handle_05::RawWindowHandle {
        let ptr = self.popup.wl_surface().id().as_ptr();
        let mut window_handle = raw_window_handle_05::WaylandWindowHandle::empty();
        window_handle.surface = ptr as *mut _;

        raw_window_handle_05::RawWindowHandle::Wayland(window_handle)
    }
}

impl<T: Application + 'static> PopupHandler for RuntimeState<T> {
    fn configure(
        &mut self,
        _: &wayland_client::Connection,
        queue_handle: &wayland_client::QueueHandle<Self>,
        popup: &XdgPopup,
        configure: PopupConfigure,
    ) {
        let winit = &mut self.winit;
        let window_id = crate::make_wid(popup.wl_surface());
        let popup = match winit.windows.get_mut(&window_id) {
            Some(Window::Popup(window)) => window,
            _ => return,
        };

        let scale_factor = popup.scale_factor;

        if let Some(initial_size) = popup.initial_size.take() {
            popup.size = initial_size.to_logical(scale_factor);
        }

        let user = self.user.as_mut().unwrap();
        let new_size = LogicalSize::<u32>::new(configure.width as u32, configure.height as u32);
        let initial_configure = popup.last_configure.is_none();

        if initial_configure {
            user.created(winit, window_id);
            user.scale_factor_changed(winit, window_id, scale_factor);
        }

        user.resized(winit, window_id, crate::logical_to_physical_rounded(new_size, scale_factor));

        if initial_configure {
            user.redraw_requested(winit, window_id);
        }
    }

    fn done(
        &mut self,
        _: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
        popup: &XdgPopup,
    ) {
        let winit = &mut self.winit;
        let window_id = crate::make_wid(popup.wl_surface());
        let popup = match winit.windows.get_mut(&window_id) {
            Some(Window::Popup(window)) => window,
            _ => return,
        };

        // todo: figure out what the hell to do here
    }
}

impl<T: Application + 'static, U> Dispatch<xdg_positioner::XdgPositioner, U> for RuntimeState<T> {
    fn event(
        state: &mut Self,
        proxy: &xdg_positioner::XdgPositioner,
        event: <xdg_positioner::XdgPositioner as wayland_client::Proxy>::Event,
        data: &U,
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // nothing, positioners do not generate events
    }
}

fn to_xdg_positioner_anchor(dir: AnchorDirection) -> xdg_positioner::Anchor {
    match dir {
        AnchorDirection::Center => xdg_positioner::Anchor::None,
        AnchorDirection::North => xdg_positioner::Anchor::Top,
        AnchorDirection::Northeast => xdg_positioner::Anchor::TopRight,
        AnchorDirection::East => xdg_positioner::Anchor::Right,
        AnchorDirection::Southeast => xdg_positioner::Anchor::BottomRight,
        AnchorDirection::South => xdg_positioner::Anchor::Bottom,
        AnchorDirection::Southwest => xdg_positioner::Anchor::BottomLeft,
        AnchorDirection::West => xdg_positioner::Anchor::Left,
        AnchorDirection::Northwest => xdg_positioner::Anchor::TopLeft,
    }
}
fn to_xdg_positioner_gravity(dir: AnchorDirection) -> xdg_positioner::Gravity {
    match dir {
        AnchorDirection::Center => xdg_positioner::Gravity::None,
        AnchorDirection::North => xdg_positioner::Gravity::Top,
        AnchorDirection::Northeast => xdg_positioner::Gravity::TopRight,
        AnchorDirection::East => xdg_positioner::Gravity::Right,
        AnchorDirection::Southeast => xdg_positioner::Gravity::BottomRight,
        AnchorDirection::South => xdg_positioner::Gravity::Bottom,
        AnchorDirection::Southwest => xdg_positioner::Gravity::BottomLeft,
        AnchorDirection::West => xdg_positioner::Gravity::Left,
        AnchorDirection::Northwest => xdg_positioner::Gravity::TopLeft,
    }
}
fn to_xdg_constraints(flags: AnchorHints) -> xdg_positioner::ConstraintAdjustment {
    let mut res = xdg_positioner::ConstraintAdjustment::None;
    if flags.contains(AnchorHints::SLIDE_X) {
        res |= xdg_positioner::ConstraintAdjustment::SlideX;
    }
    if flags.contains(AnchorHints::SLIDE_Y) {
        res |= xdg_positioner::ConstraintAdjustment::SlideY;
    }
    if flags.contains(AnchorHints::FLIP_X) {
        res |= xdg_positioner::ConstraintAdjustment::FlipX;
    }
    if flags.contains(AnchorHints::FLIP_Y) {
        res |= xdg_positioner::ConstraintAdjustment::FlipY;
    }
    if flags.contains(AnchorHints::RESIZE_X) {
        res |= xdg_positioner::ConstraintAdjustment::ResizeX;
    }
    if flags.contains(AnchorHints::RESIZE_Y) {
        res |= xdg_positioner::ConstraintAdjustment::ResizeY;
    }
    res
}
