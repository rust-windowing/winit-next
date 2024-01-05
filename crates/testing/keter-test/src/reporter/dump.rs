// MIT/Apache2 License

//! Dump JSON output to the console.
//!
//! This is needed for transmitting details for the Android implementation,
//! unfortunately.

use super::Reporter;

/// Report tests to the output console in JSON form.
pub struct DumpReporter;

impl DumpReporter {
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Reporter for DumpReporter {
    #[inline]
    fn report(
        &mut self,
        test: crate::TestEvent,
    ) -> std::pin::Pin<Box<dyn futures_lite::prelude::Future<Output = ()> + Send + '_>> {
        Box::pin(blocking::unblock(move || {
            let data = serde_json::to_string(&test).unwrap();
            println!("KETER_TEST_DUMP({data})KETER_TEST_DUMP");
        }))
    }

    #[inline]
    fn finish(&mut self) -> i32 {
        0
    }
}
