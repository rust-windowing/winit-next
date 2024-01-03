use crate::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MonitorId(pub u128);

pub trait Monitor {
    /// Return the given monitor id.
    fn id(&self) -> MonitorId;
    /// Returns a human-readable name of the monitor.
    ///
    /// Returns `None` if the monitor doesn't exist anymore.
    fn name(&self) -> Option<String>;

    /// Returns the monitor's resolution.
    fn size(&self) -> PhysicalSize<u32>;

    /// Returns the top-left corner position of the monitor relative to the
    /// larger full screen area.
    fn position(&self) -> PhysicalPosition<i32>;

    /// The monitor refresh rate used by the system.
    ///
    /// Return `Some` if succeed, or `None` if failed, which usually happens
    /// when the monitor the window is on is removed.
    ///
    /// When using exclusive fullscreen, the refresh rate of the
    /// [`VideoModeHandle`] that was used to enter fullscreen should be used
    /// instead.
    fn refresh_rate_millihertz(&self) -> Option<u32>;

    fn scale_factor(&self) -> f64;
}
