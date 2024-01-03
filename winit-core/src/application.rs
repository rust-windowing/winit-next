use std::time::Instant;

use crate::dpi::PhysicalSize;
use crate::event_loop::EventLoopHandle;
use crate::input::touch::TouchInputHandler;
use crate::window::WindowId;

pub trait Application: ApplicationWindow {
    /// Emitted when new events arrive from the OS to be processed.
    fn new_events(&mut self, loop_handle: &mut dyn EventLoopHandle, start_cause: StartCause);

    /// Emitted when the event loop is about to block and wait for new events.
    fn about_to_wait(&mut self, loop_handle: &mut dyn EventLoopHandle);

    /// Emitted when the event loop is being shut down.
    fn loop_exiting(&mut self, loop_handle: &mut dyn EventLoopHandle);

    // The APIs which we consider optional, thus the application may opt-in/out the
    // behavior. XXX ========================= ¯\_(ツ)_/¯
    // ================================

    #[inline(always)]
    fn touch_handler(&mut self) -> Option<&mut dyn TouchInputHandler> {
        None
    }

    #[inline(always)]
    fn device_events_handelr(&mut self) -> Option<&mut dyn DeviceEventsHandler> {
        None
    }
}

pub trait ApplicationWindow {
    /// The window with the given `window_id` was created.
    fn created(&mut self, loop_handle: &mut dyn EventLoopHandle, window_id: WindowId);

    /// The size of the window has changed. Contains the client area's new
    /// dimensions.
    fn resized(
        &mut self,
        loop_handle: &mut dyn EventLoopHandle,
        window_id: WindowId,
        size: PhysicalSize<u32>,
    );

    /// The window's scale factor has changed.
    fn scale_factor_changed(
        &mut self,
        loop_handle: &mut dyn EventLoopHandle,
        window_id: WindowId,
        scale_factor: f64,
    );

    /// Emitted when a window should be redrawn.
    fn redraw_requested(&mut self, loop_handle: &mut dyn EventLoopHandle, window_id: WindowId);

    /// The window has been requested to close.
    fn close_requested(
        &mut self,
        loop_handle: &mut dyn EventLoopHandle,
        window_id: WindowId,
    ) -> bool;

    /// The window gained or lost focus.
    fn focused(
        &mut self,
        loop_handle: &mut dyn EventLoopHandle,
        window_id: WindowId,
        focused: bool,
    ) {
        let _ = loop_handle;
        let _ = window_id;
        let _ = focused;
    }

    /// The window has been occluded (completely hidden from view).
    ///
    /// This is different to window visibility as it depends on whether the
    /// window is closed, minimised, set invisible, or fully occluded by
    /// another window.
    fn occluded(
        &mut self,
        loop_handle: &mut dyn EventLoopHandle,
        window_id: WindowId,
        occluded: bool,
    ) {
        let _ = loop_handle;
        let _ = window_id;
        let _ = occluded;
    }

    /// The window has been destroyed.
    fn destroyed(&mut self, loop_handle: &mut dyn EventLoopHandle, window_id: WindowId) {
        let _ = loop_handle;
        let _ = window_id;
    }
}

pub trait DeviceEventsHandler: Application {}

/// Describes the reason the event loop is resuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCause {
    /// Sent if the time specified by [`ControlFlow::WaitUntil`] has been
    /// reached. Contains the moment the timeout was requested and the
    /// requested resume time. The actual resume time is guaranteed to be
    /// equal to or after the requested resume time.
    ///
    /// [`ControlFlow::WaitUntil`]: crate::event_loop::ControlFlow::WaitUntil
    ResumeTimeReached { start: Instant, requested_resume: Instant },

    /// Sent if the OS has new events to send to the window, after a wait was
    /// requested. Contains the moment the wait was requested and the resume
    /// time, if requested.
    WaitCancelled { start: Instant, requested_resume: Option<Instant> },

    /// Sent if the event loop is being resumed after the loop's control flow
    /// was set to [`ControlFlow::Poll`].
    ///
    /// [`ControlFlow::Poll`]: crate::event_loop::ControlFlow::Poll
    Poll,

    /// Sent once, immediately after `run` is called. Indicates that the loop
    /// was just initialized.
    Init,
}
