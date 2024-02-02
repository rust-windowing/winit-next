use std::num::NonZeroU32;
use std::time::Duration;

use winit_core::application::{Application, ApplicationWindow, StartCause};
use winit_core::dpi::PhysicalSize;
use winit_core::event_loop::{EventLoopHandle, EventLoopRequests};
use winit_core::window::WindowId;
use winit_wayland::event_loop::EventLoop;
use winit_wayland::MyCoolTrait;

use softbuffer::{Context, Surface};

const DARK_GRAY: u32 = 0xFF181818;

pub struct State {
    context: Context,
    surface: Option<Surface>,
}

// TODO: It's not clear how to do user events with all of that, for example if
// we add associated type it'll bleed through out the codebase. Maybe what we
// should do is to develop a way to wake up the loop, thus the actual user
// events will be squashed when needed, since wakeup is squashed, and how the
// data is transmitted, etc will be up to users.
//
// In general, a generic interface to wakeup the loop and then the user can
// `poll` the sources looks more appealing.

impl MyCoolTrait for State {
    fn foo(&mut self) {
        println!("Hello from user trait!");
    }
}

impl Application for State {
    fn user_wakeup(&mut self, _: &mut dyn EventLoopHandle) {
        println!("Wake up");
    }

    fn new_events(&mut self, loop_handle: &mut dyn EventLoopHandle, start_cause: StartCause) {
        println!("Start cause {start_cause:?}");
        let _ = loop_handle.create_window(&Default::default());
    }

    fn about_to_wait(&mut self, _: &mut dyn EventLoopHandle) {
        println!("About to wait");
    }

    fn loop_exiting(&mut self, _: &mut dyn EventLoopHandle) {
        println!("Exiting the loop");
    }

    fn as_any(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

use winit_wayland::window::Window as WaylandWindow;

impl ApplicationWindow for State {
    fn created(&mut self, loop_handle: &mut dyn EventLoopHandle, window_id: WindowId) {
        let window = loop_handle.get_window(window_id).unwrap();
        self.surface = unsafe {
            Some(Surface::new(&self.context, &window).expect("failed to create surface"))
        };
    }

    fn close_requested(&mut self, _: &mut dyn EventLoopHandle, _: WindowId) -> bool {
        true
    }

    fn resized(
        &mut self,
        loop_handle: &mut dyn EventLoopHandle,
        window_id: WindowId,
        size: PhysicalSize<u32>,
    ) {
        println!("New size {size:?}");
        let window = loop_handle.get_window_mut(window_id).unwrap();
        window.request_redraw();
        window.as_any().downcast_mut::<WaylandWindow>().unwrap().out_of_tree_method();
    }

    fn scale_factor_changed(
        &mut self,
        _: &mut dyn EventLoopHandle,
        _: WindowId,
        scale_factor: f64,
    ) {
        println!("New scale factor {scale_factor}");
    }

    fn redraw_requested(&mut self, loop_handle: &mut dyn EventLoopHandle, window_id: WindowId) {
        let (window, surface) = match (loop_handle.get_window(window_id), self.surface.as_mut()) {
            (Some(window), Some(surface)) => (window, surface),
            _ => return,
        };

        if let Some(monitor_id) = window.current_monitor() {
            let monitor = loop_handle.get_monitor(monitor_id).unwrap();
            println!("Current monitor name {:?}", monitor.name());
        }

        let size = window.inner_size();
        let _ = surface
            .resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap());
        let mut buffer = surface.buffer_mut().unwrap();
        buffer.fill(DARK_GRAY);
        buffer.present().unwrap();
    }

    fn destroyed(&mut self, loop_handle: &mut dyn EventLoopHandle, _: WindowId) {
        if loop_handle.num_windows() == 0 {
            loop_handle.exit();
        }
    }
}

fn main() {
    // TODO this is ugly.
    let mut event_loop = <EventLoop as EventLoopRequests<State>>::new().unwrap();
    let context =
        unsafe { Context::new(&event_loop).expect("failed to create softbuffer context") };
    let state = State { context, surface: None };

    let proxy = EventLoopRequests::<State>::proxy(&mut event_loop);

    event_loop.setup_my_cool_trait_handler::<State>();

    // Test out the proxy.
    std::thread::spawn(move || loop {
        proxy.wakeup();
        std::thread::sleep(Duration::from_millis(500));
    });

    event_loop.run(state);
}
