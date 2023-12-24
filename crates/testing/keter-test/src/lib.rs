// MIT/Apache2 License

//! A testing framework designed to be used internally in `keter`.

pub mod reporter;

use async_lock::Mutex;
use futures_lite::prelude::*;

use std::borrow::Cow;
use std::future::Future;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The event of a test.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum TestEvent {
    /// There are no more tests.
    End {
        /// The total number of tests.
        count: usize,
    },

    /// Begin a test group.
    BeginGroup {
        /// Name of the test group.
        name: Cow<'static, str>,

        /// Number of tests.
        count: usize,
    },

    /// End a test group.
    EndGroup(Cow<'static, str>),

    /// The result of a test.
    Result(TestResult),
}

/// The result of the test.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TestResult {
    /// The name of the test.
    pub name: Cow<'static, str>,

    /// The status of the test.
    pub status: TestStatus,

    /// Description of the test failure.
    pub failure: Cow<'static, str>,
}

/// The status of the test.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum TestStatus {
    /// The test succeeded.
    Success,

    /// The test failed.
    Failed,

    /// The test was ignored.
    Ignored,
}

/// The test harness.
pub struct TestHarness {
    reporter: Mutex<Box<dyn reporter::Reporter + Send + 'static>>,
    count: AtomicUsize,
}

impl TestHarness {
    /// Run with a test group.
    pub async fn group(&self, name: impl Into<String>, count: usize, f: impl Future<Output = ()>) {
        let name = name.into();
        self.reporter
            .lock()
            .await
            .report(TestEvent::BeginGroup {
                name: name.clone().into(),
                count,
            })
            .await;
        f.await;
        self.reporter
            .lock()
            .await
            .report(TestEvent::EndGroup(name.into()))
            .await;
    }

    /// Run a test.
    pub async fn test(&self, name: impl Into<String>, f: impl Future<Output = ()>) {
        let name = name.into();
        self.count.fetch_add(1, Ordering::Relaxed);

        let result = { panic::AssertUnwindSafe(f).catch_unwind().await };

        match result {
            Ok(()) => {
                self.reporter
                    .lock()
                    .await
                    .report(TestEvent::Result(TestResult {
                        name: name.into(),
                        status: TestStatus::Success,
                        failure: "".into(),
                    }))
                    .await;
            }

            Err(err) => {
                let failure: Cow<'static, str> = if let Some(e) = err.downcast_ref::<&'static str>()
                {
                    (*e).into()
                } else if let Ok(e) = err.downcast::<String>() {
                    (*e).into()
                } else {
                    "<unintelligible panic>".into()
                };

                self.reporter
                    .lock()
                    .await
                    .report(TestEvent::Result(TestResult {
                        name: name.into(),
                        status: TestStatus::Failed,
                        failure,
                    }))
                    .await;
            }
        }
    }
}

/// Run tests with a harness.
pub fn run_tests<T>(f: impl FnOnce(&TestHarness) -> T) -> T {
    // Set up hooks.
    human_panic::setup_panic!();
    tracing_subscriber::fmt::try_init().ok();
    color_eyre::install().ok();

    // Create our test harness.
    // TODO: Other reporters.
    let harness = TestHarness {
        reporter: Mutex::new(Box::new(reporter::ConsoleReporter::new())),
        count: AtomicUsize::new(0),
    };

    // Run the tests.
    let value = f(&harness);

    // Count tests.
    let TestHarness { reporter, count } = harness;
    let mut reporter = reporter.into_inner();
    futures_lite::future::block_on(reporter.report(TestEvent::End {
        count: count.into_inner(),
    }));

    // Finish with an exit code.
    let code = reporter.finish();
    if code != 0 {
        std::process::exit(code);
    }

    value
}
