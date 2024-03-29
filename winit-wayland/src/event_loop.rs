use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use calloop::ping::Ping;
use calloop::LoopHandle;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use raw_window_handle_05::HasRawDisplayHandle as HasRawDisplayHandle05;

use sctk::reexports::calloop_wayland_source::WaylandSource;
use sctk::reexports::client::globals::GlobalList;
use sctk::reexports::client::protocol::wl_output::WlOutput;
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::{globals, Connection, QueueHandle};
use sctk::registry::{ProvidesRegistryState, RegistryState};

use sctk::output::{OutputHandler, OutputState};
use sctk::seat::{Capability as SeatCapability, SeatHandler, SeatState};

use winit_core::application::Application;
use winit_core::event_loop::proxy::EventLoopProxy as CoreEventLoopProxy;
use winit_core::event_loop::{EventLoopHandle, EventLoopRequests};
use winit_core::window::{Window as CoreWindow, WindowId};

use crate::state::WinitState;
use crate::MyCoolTrait;

pub struct EventLoop<T: Application + 'static> {
    state: RuntimeState<T>,

    event_loop: calloop::EventLoop<'static, RuntimeState<T>>,
}

impl<T: Application + 'static> EventLoopRequests<T> for EventLoop<T> {
    fn new() -> Result<Self, ()> {
        let connection = Connection::connect_to_env().unwrap();

        let (globals, mut event_queue) = globals::registry_queue_init(&connection).unwrap();
        let queue_handle = event_queue.handle();

        let event_loop = calloop::EventLoop::<RuntimeState<T>>::try_new().unwrap();

        // Insert the proxy source.
        let (ping, ping_source) = calloop::ping::make_ping().unwrap();
        let proxy = EventLoopProxy::new(ping);

        let _ =
            event_loop.handle().insert_source(ping_source, |_, _, state: &mut RuntimeState<T>| {
                let winit = &mut state.winit;
                let user = &mut state.user.as_mut().unwrap();
                user.user_wakeup(winit);
            });

        let mut state = RuntimeState {
            user: None,
            winit: WinitState::new(connection.clone(), &globals, &queue_handle, proxy).unwrap(),
            vtable: Vtable::default(),
        };

        let _ = event_queue.roundtrip(&mut state);

        let wayland_source = WaylandSource::new(connection, event_queue);
        wayland_source.insert(event_loop.handle()).unwrap();

        Ok(Self { event_loop, state })
    }

    fn run(mut self, mut state: T) {
        // SAFETY: The user state is being used only inside the loop and can't have
        // Wayland objects in it. Calloop itself allow the state to have a
        // non-static lifetime attached to it, however wayland-rs forces bound
        // by static. The wayland objects are stored on a bound by
        // static `winit` state owned by the event loop, and user application trait has
        // a lifetime of `'a` allowed by calloop, thus making it sound if you
        // think that calloop dispatches not one but 2 states at the same time.
        self.state.user =
            Some(unsafe { std::mem::transmute::<&mut T, &'static mut T>(&mut state) });

        self.state
            .user
            .as_mut()
            .unwrap()
            .new_events(&mut self.state.winit, winit_core::application::StartCause::Init);

        let mut redraw = Vec::new();

        loop {
            if self.state.winit.connection.flush().is_err() {
                return;
            }

            let winit = &mut self.state.winit;
            let user = self.state.user.as_mut().unwrap();

            for (window_id, window) in &mut winit.windows {
                if mem::take(&mut window.redraw) {
                    redraw.push(*window_id);
                }
            }

            // Issue synthetic redraws issued by users.
            for window_id in redraw.drain(..) {
                user.redraw_requested(winit, window_id)
            }

            self.state.user.as_mut().unwrap().about_to_wait(&mut self.state.winit);

            // TODO: we should handle waking up for the next iteration due to
            // redraw-requested here.

            self.event_loop.dispatch(None, &mut self.state).unwrap();

            if self.state.winit.exit {
                break;
            }
        }

        self.state.user.as_mut().unwrap().loop_exiting(&mut self.state.winit);
    }

    fn proxy(&self) -> Arc<dyn CoreEventLoopProxy> {
        self.state.winit.proxy()
    }
}

impl<T: Application + 'static + MyCoolTrait> EventLoop<T> {
    /// This sets up handelr for `MyCoolTrait` but doesn't force it through-out the codebase.
    pub fn register_my_cool_trait_handler(&mut self) {
        self.state.vtable.foo = Some(T::foo);
    }
}

impl<T: Application + 'static> HasDisplayHandle for EventLoop<T> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.state.winit.display_handle()
    }
}

unsafe impl<T: Application + 'static> HasRawDisplayHandle05 for EventLoop<T> {
    fn raw_display_handle(&self) -> raw_window_handle_05::RawDisplayHandle {
        self.state.winit.raw_display_handle()
    }
}

/// Runtime state passed around.
pub(crate) struct RuntimeState<T: Application + 'static> {
    /// The user state we're using during the runtime.
    pub user: Option<&'static mut T>,

    /// The state of the winit.
    pub winit: WinitState<T>,

    pub vtable: Vtable<T>,
}

pub struct Vtable<T: Application + 'static> {
    pub foo: Option<fn(&mut T)>,
}

impl<T: Application + 'static> Default for Vtable<T> {
    fn default() -> Self {
        Self { foo: None }
    }
}

pub struct EventLoopProxy {
    ping: Ping,
}

impl EventLoopProxy {
    fn new(ping: Ping) -> Self {
        Self { ping }
    }
}

impl CoreEventLoopProxy for EventLoopProxy {
    fn wakeup(&self) {
        self.ping.ping();
    }
}
