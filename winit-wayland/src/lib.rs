#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use sctk::reexports::client::protocol::wl_output::WlOutput;
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::Proxy;

use winit_core::dpi::{LogicalSize, PhysicalSize};
use winit_core::monitor::MonitorId;
use winit_core::window::WindowId;

pub mod event_loop;
pub mod monitor;
pub mod state;
pub mod window;

/// Get the WindowId out of the surface.
#[inline]
pub(crate) fn make_wid(surface: &WlSurface) -> WindowId {
    WindowId(surface.id().as_ptr() as u128)
}

/// Get the WindowId out of the surface.
#[inline]
pub(crate) fn make_mid(output: &WlOutput) -> MonitorId {
    MonitorId(output.id().as_ptr() as u128)
}

/// The default routine does floor, but we need round on Wayland.
pub(crate) fn logical_to_physical_rounded(
    size: LogicalSize<u32>,
    scale_factor: f64,
) -> PhysicalSize<u32> {
    let width = size.width as f64 * scale_factor;
    let height = size.height as f64 * scale_factor;
    (width.round(), height.round()).into()
}
