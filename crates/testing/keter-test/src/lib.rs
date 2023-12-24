// MIT/Apache2 License

//! A testing framework designed to be used internally in `keter`.

pub mod reporter;

use std::borrow::Cow;

/// The result of a test.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum TestResult {
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
    Result {
        /// The name of the test.
        name: Cow<'static, str>,

        /// The status of the test.
        status: TestStatus,

        /// Description of the test failure.
        failure: Cow<'static, str>,
    },
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

/// Run the test harness.
pub fn run_tests() {}
