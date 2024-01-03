use std::collections::HashMap;
use std::mem;

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
use winit_core::event_loop::{EventLoopHandle, EventLoopRequests};
use winit_core::window::{Window as CoreWindow, WindowId};

use crate::state::WinitState;

pub struct EventLoop {
    state: RuntimeState,

    event_loop: calloop::EventLoop<'static, RuntimeState>,
}

impl<D> EventLoopRequests<D> for EventLoop
where
    D: Application + 'static,
{
    fn new() -> Result<Self, ()> {
        let connection = Connection::connect_to_env().unwrap();

        let (globals, mut event_queue) = globals::registry_queue_init(&connection).unwrap();
        let queue_handle = event_queue.handle();

        let event_loop = calloop::EventLoop::<RuntimeState>::try_new().unwrap();
        let mut state = RuntimeState {
            user: None,
            winit: WinitState::new(connection.clone(), &globals, &queue_handle).unwrap(),
        };

        let _ = event_queue.roundtrip(&mut state);

        let wayland_source = WaylandSource::new(connection, event_queue);
        wayland_source.insert(event_loop.handle()).unwrap();

        Ok(Self { event_loop, state })
    }

    fn run(mut self, mut state: D) {
        // TODO, yesu-yesu, very safu.
        self.state.user =
            Some(unsafe { std::mem::transmute::<&mut D, &'static mut D>(&mut state) });

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

            for window_id in redraw.drain(..) {
                user.redraw_requested(winit, window_id)
            }

            self.state.user.as_mut().unwrap().about_to_wait(&mut self.state.winit);

            self.event_loop.dispatch(None, &mut self.state).unwrap();

            if self.state.winit.exit {
                break;
            }
        }

        self.state.user.as_mut().unwrap().loop_exiting(&mut self.state.winit);
    }
}

impl HasDisplayHandle for EventLoop {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.state.winit.display_handle()
    }
}

unsafe impl HasRawDisplayHandle05 for EventLoop {
    fn raw_display_handle(&self) -> raw_window_handle_05::RawDisplayHandle {
        self.state.winit.raw_display_handle()
    }
}

/// Runtime state passed around.
pub(crate) struct RuntimeState {
    /// The user state we're using during the runtime.
    pub user: Option<&'static mut dyn Application>,

    /// The state of the winit.
    pub winit: WinitState,
}
