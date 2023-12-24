// MIT/Apache2 License

//! The trait for reporting error results.

use super::TestResult;

mod console;

use console::ConsoleReporter;

/// Something that receives test results.
pub trait Reporter {
    /// Report a test event.
    fn report(&mut self, test: TestResult);

    /// Finish our report, returning an exit code.
    fn finish(&mut self) -> i32;
}
