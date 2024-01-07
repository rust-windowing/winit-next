use std::collections::HashMap;
use std::sync::Arc;

use calloop::ping::Ping;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use raw_window_handle_05::HasRawDisplayHandle as HasRawDisplayHandle05;

use sctk::reexports::calloop::LoopHandle;
use sctk::reexports::client::backend::ObjectId;
use sctk::reexports::client::globals::GlobalList;
use sctk::reexports::client::protocol::wl_output::{self, WlOutput};
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::{Connection, Proxy, QueueHandle};

use sctk::compositor::{CompositorHandler, CompositorState};
use sctk::output::{OutputHandler, OutputState};
use sctk::registry::{ProvidesRegistryState, RegistryState};
use sctk::seat::pointer::ThemedPointer;
use sctk::seat::{Capability as SeatCapability, SeatHandler, SeatState};
use sctk::shell::xdg::window::{Window as XdgWindow, WindowConfigure, WindowHandler};
use sctk::shell::xdg::XdgShell;
use sctk::shell::WaylandSurface;
use sctk::shm::slot::SlotPool;
use sctk::shm::{Shm, ShmHandler};
use sctk::subcompositor::SubcompositorState;

use winit_core::event_loop::proxy::EventLoopProxy as CoreEventLoopProxy;
use winit_core::event_loop::EventLoopHandle;
use winit_core::monitor::{Monitor as CoreMonitor, MonitorId};
use winit_core::window::{Window as CoreWindow, WindowAttributes, WindowId};

use crate::monitor::Monitor;
use crate::window::Window;

use crate::event_loop::{EventLoopProxy, RuntimeState};

impl EventLoopHandle for WinitState {
    fn proxy(&self) -> Arc<dyn CoreEventLoopProxy> {
        self.proxy.clone()
    }

    fn create_window(&mut self, attributes: &WindowAttributes) -> Result<(), ()> {
        let window = Window::new(self, attributes);
        let window_id = window.id();
        self.windows.insert(window_id, window);
        Ok(())
    }

    fn num_windows(&self) -> usize {
        self.windows.len()
    }

    fn get_window(&self, window_id: WindowId) -> Option<&dyn CoreWindow> {
        let window = self.windows.get(&window_id)?;

        if window.last_configure.is_none() {
            return None;
        } else {
            Some(window as &dyn CoreWindow)
        }
    }

    fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut dyn CoreWindow> {
        let window = self.windows.get_mut(&window_id)?;
        if window.last_configure.is_none() {
            return None;
        } else {
            Some(window as &mut dyn CoreWindow)
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn get_monitor(&self, monitor_id: MonitorId) -> Option<&dyn CoreMonitor> {
        self.monitors
            .iter()
            .find(|monitor| monitor.id() == monitor_id)
            .map(|monitor| monitor as &dyn CoreMonitor)
    }

    fn monitors(&self) -> Vec<&dyn CoreMonitor> {
        self.monitors.iter().map(|monitor| monitor as &dyn CoreMonitor).collect()
    }
}

impl HasDisplayHandle for WinitState {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        todo!()
    }
}

unsafe impl HasRawDisplayHandle05 for WinitState {
    fn raw_display_handle(&self) -> raw_window_handle_05::RawDisplayHandle {
        let mut display_handle = raw_window_handle_05::WaylandDisplayHandle::empty();
        display_handle.display = self.connection.display().id().as_ptr() as *mut _;
        raw_window_handle_05::RawDisplayHandle::Wayland(display_handle)
    }
}

/// Winit's Wayland state.
pub struct WinitState {
    /// The underlying connection.
    pub connection: Connection,

    /// The WlRegistry.
    pub registry_state: RegistryState,

    /// The seat state responsible for all sorts of input.
    pub seat_state: SeatState,

    /// The state of the WlOutput handling.
    pub output_state: OutputState,

    /// The compositor state which is used to create new windows and regions.
    pub compositor: Arc<CompositorState>,

    /// The state of the subcompositor.
    pub subcompositor: Option<Arc<SubcompositorState>>,

    /// The shm for software buffers, such as cursors.
    pub shm: Shm,

    /// The XDG shell that is used for widnows.
    pub xdg_shell: XdgShell,

    /// Currently handled seats.
    pub seats: HashMap<ObjectId, ()>,

    pub windows: HashMap<WindowId, Window>,

    pub monitors: Vec<Monitor>,

    pub(crate) queue_handle: QueueHandle<RuntimeState>,

    pub proxy: Arc<EventLoopProxy>,

    pub exit: bool,
}

impl WinitState {
    pub(crate) fn new(
        connection: Connection,
        globals: &GlobalList,
        queue_handle: &QueueHandle<RuntimeState>,
        proxy: EventLoopProxy,
    ) -> Result<Self, ()> {
        let registry_state = RegistryState::new(globals);
        let output_state = OutputState::new(globals, queue_handle);

        let seat_state = SeatState::new(globals, queue_handle);
        let mut seats = HashMap::default();
        for seat in seat_state.seats() {
            seats.insert(seat.id(), ());
        }

        let compositor_state = Arc::new(CompositorState::bind(globals, queue_handle).unwrap());
        let subcompositor_state = match SubcompositorState::bind(
            compositor_state.wl_compositor().clone(),
            globals,
            queue_handle,
        ) {
            Ok(subcompositor) => Some(Arc::new(subcompositor)),
            Err(err) => {
                // eprintln!("Subcompositor protocol not available, ignoring CSD: {err:?}");
                None
            },
        };

        let shm = Shm::bind(globals, queue_handle).unwrap();
        let monitors = output_state.outputs().map(Monitor::new).collect();

        Ok(Self {
            xdg_shell: XdgShell::bind(globals, queue_handle).unwrap(),
            queue_handle: queue_handle.clone(),
            subcompositor: subcompositor_state,
            compositor: compositor_state,
            proxy: Arc::new(proxy),
            registry_state,
            output_state,
            seat_state,
            connection,
            monitors,
            seats,
            shm,
            windows: Default::default(),
            exit: Default::default(),
        })
    }

    pub(crate) fn scale_factor_changed(
        state: &mut RuntimeState,
        surface: &WlSurface,
        scale_factor: f64,
        legacy: bool,
    ) {
        let winit = &mut state.winit;
        let window_id = crate::make_wid(surface);
        let window = match winit.windows.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        window.set_scale_factor(scale_factor);

        let user_state = &mut state.user.as_mut().unwrap();

        // Only send scale for configured windows.
        if window.configured() {
            user_state.scale_factor_changed(winit, window_id, scale_factor);
        }
    }
}

impl ProvidesRegistryState for RuntimeState {
    sctk::registry_handlers![OutputState, SeatState];

    fn registry(&mut self) -> &mut RegistryState {
        &mut self.winit.registry_state
    }
}

impl SeatHandler for RuntimeState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.winit.seat_state
    }

    fn new_capability(
        &mut self,
        _: &Connection,
        queue_handle: &QueueHandle<Self>,
        seat: WlSeat,
        capability: SeatCapability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _queue_handle: &QueueHandle<Self>,
        seat: WlSeat,
        capability: SeatCapability,
    ) {
    }

    fn new_seat(
        &mut self,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
        seat: WlSeat,
    ) {
    }

    fn remove_seat(
        &mut self,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
        seat: WlSeat,
    ) {
    }
}

impl CompositorHandler for RuntimeState {
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        scale_factor: i32,
    ) {
        WinitState::scale_factor_changed(self, surface, scale_factor as f64, true)
    }

    fn transform_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_transform: wl_output::Transform,
    ) {
        // TODO(kchibisov) we need to expose it somehow in winit.
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, time: u32) {
        todo!()
    }
}

impl ShmHandler for RuntimeState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.winit.shm
    }
}

sctk::delegate_registry!(RuntimeState);
sctk::delegate_seat!(RuntimeState);
sctk::delegate_subcompositor!(RuntimeState);
sctk::delegate_shm!(RuntimeState);
sctk::delegate_compositor!(RuntimeState);
sctk::delegate_xdg_shell!(RuntimeState);
sctk::delegate_xdg_window!(RuntimeState);
