// MIT/Apache2 License

//! The trait for reporting error results.

use super::TestEvent;

mod console;

pub use console::ConsoleReporter;

/// Something that receives test results.
pub trait Reporter {
    /// Report a test event.
    fn report(&mut self, test: TestEvent);

    /// Finish our report, returning an exit code.
    fn finish(&mut self) -> i32;
}
