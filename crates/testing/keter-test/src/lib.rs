// MIT/Apache2 License

//! A testing framework designed to be used internally in `keter`.

pub mod reporter;

use std::borrow::Cow;
use std::panic;

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
    reporter: Box<dyn reporter::Reporter + Send + 'static>,
    count: usize,
}

impl TestHarness {
    /// Run with a test group.
    pub fn group(&mut self, name: impl Into<String>, count: usize, f: impl FnOnce(&mut TestHarness)) {
        let name = name.into();
        self.reporter.report(TestEvent::BeginGroup { name: name.clone().into(), count });
        f(self);
        self.reporter.report(TestEvent::EndGroup(name.into()));
    }

    /// Run a test.
    pub fn test(&mut self, name: impl Into<String>, f: impl FnOnce(&mut TestHarness)) {
        let name = name.into();
        self.count += 1;

        match panic::catch_unwind(panic::AssertUnwindSafe(|| f(self))) {
            Ok(()) => {
                self.reporter.report(TestEvent::Result(TestResult {
                    name: name.into(),
                    status: TestStatus::Success,
                    failure: "".into()
                }));
            }

            Err(err) => {
                let failure: Cow<'static, str> = if let Some(e) = err.downcast_ref::<&'static str>() {
                    (*e).into()
                } else if let Ok(e) = err.downcast::<String>() {
                    (*e).into()
                } else {
                    "<unintelligible panic>".into()
                };

                self.reporter.report(TestEvent::Result(TestResult {
                    name: name.into(),
                    status: TestStatus::Failed,
                    failure
                }));
            }
        }
    }
}

/// Run tests with a harness.
pub fn run_tests(f: impl FnOnce(&mut TestHarness)) -> ! {
    // Set up hooks.
    human_panic::setup_panic!();
    tracing_subscriber::fmt::try_init().ok();
    color_eyre::install().ok();

    // Create our test harness.
    // TODO: Other reporters.
    let mut harness = TestHarness {
        reporter: Box::new(reporter::ConsoleReporter::new()),
        count: 0
    };

    // Run the tests.
    f(&mut harness);

    // Finish with an exit code.
    std::process::exit(harness.reporter.finish())
}
