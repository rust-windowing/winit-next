use raw_window_handle::HasDisplayHandle;
use raw_window_handle_05::HasRawDisplayHandle as HasRawDisplayHandle05;

use crate::application::Application;
use crate::monitor::{Monitor, MonitorId};
use crate::window::{Window, WindowAttributes, WindowId};

/// API to run the event loop.
pub struct EventLoop {}

pub trait EventLoopRequests<D>: HasDisplayHandle + HasRawDisplayHandle05 + Sized
where
    D: Application + 'static,
{
    fn new() -> Result<Self, ()>;

    /// Run the event loop.
    fn run(self, state: D);
}

/// Handle for the event loop.
pub trait EventLoopHandle: HasDisplayHandle {
    /// Request to create a window.
    fn create_window(&mut self, attributes: &WindowAttributes) -> Result<(), ()>;

    fn num_windows(&self) -> usize;

    fn get_window(&self, window_id: WindowId) -> Option<&dyn Window>;

    fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut dyn Window>;

    fn get_monitor(&self, monitor_id: MonitorId) -> Option<&dyn Monitor>;

    fn monitors(&self) -> Vec<&dyn Monitor>;

    fn exit(&mut self);
}
