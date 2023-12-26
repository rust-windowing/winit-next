// MIT/Apache2 License

//! The trait for reporting error results.

use super::TestEvent;

use std::future::Future;
use std::pin::Pin;

mod console;
mod dump;
mod writer;

pub use console::ConsoleReporter;
pub use dump::DumpReporter;
pub use writer::StreamReporter;

/// Something that receives test results.
pub trait Reporter {
    /// Report a test event.
    fn report(&mut self, test: TestEvent) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Finish our report, returning an exit code.
    fn finish(&mut self) -> i32;
}
