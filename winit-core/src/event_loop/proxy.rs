/// Proxy to wake up the event loop from non-main thread.
pub trait EventLoopProxy: Send + Sync {
    /// Wakeup the event loop.
    fn wakeup(&self);
}
