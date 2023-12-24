// MIT/Apache2

use super::Reporter;
use crate::{TestEvent, TestStatus, TestResult};

use owo_colors::OwoColorize;
use std::borrow::Cow;
use std::fmt::{self, Write as _};
use std::io::{self, prelude::*};

/// Report tests to the output console.
pub struct ConsoleReporter {
    /// The current exit code.
    exit_code: i32,

    /// Current indentation.
    indent: usize,

    /// Look for test failures.
    failures: Vec<(Cow<'static, str>, Cow<'static, str>)>,
}

impl Default for ConsoleReporter {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleReporter {
    /// Create a new `ConsoleReporter`.
    #[inline]
    pub fn new() -> Self {
        Self {
            exit_code: 0,
            indent: 0,
            failures: vec![],
        }
    }
}

impl Reporter for ConsoleReporter {
    #[inline]
    fn report(&mut self, test: TestEvent) {
        let mut cout = io::stdout().lock();

        match test {
            TestEvent::End { count: _ } => {
                if self.exit_code == 0 {
                    writeln!(cout, "{}{}", "test result: ".white(), "ok".green()).unwrap();
                } else {
                    writeln!(cout, "{}{}", "test result: ".white(), "FAILED".red()).unwrap();
                }
            }
            TestEvent::BeginGroup { name, count } => {
                writeln!(
                    cout,
                    "{}{}{}{}{}{}",
                    Indent(self.indent),
                    "running test group '".white().italic(),
                    name.cyan().bold(),
                    "' with ".white().italic(),
                    count.cyan().bold(),
                    " tests...".white().italic()
                )
                .unwrap();

                self.indent += 1;
            }
            TestEvent::EndGroup(_name) => {}
            TestEvent::Result(TestResult {
                name,
                status,
                failure,
            }) => {
                write!(
                    cout,
                    "{}{}{}{}",
                    Indent(self.indent),
                    "test ".white(),
                    name.bold().white(),
                    "... ".white()
                )
                .unwrap();

                match status {
                    TestStatus::Failed => {
                        self.failures.push((name, failure));
                        self.exit_code = 1;
                        writeln!(cout, "{}", "ok".green().bold()).unwrap();
                    }

                    TestStatus::Ignored => {
                        writeln!(cout, "{}", "ignored".yellow().bold()).unwrap();
                    }

                    TestStatus::Success => {
                        writeln!(cout, "{}", "ok".green().bold()).unwrap();
                    }
                }
            }
        }
    }

    #[inline]
    fn finish(&mut self) -> i32 {
        self.exit_code
    }
}

const SPACES_PER_INDENT: usize = 2;

struct Indent(usize);

impl fmt::Display for Indent {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let spaces = self.0 * SPACES_PER_INDENT;
        for _ in 0..spaces {
            f.write_char(' ')?;
        }
        Ok(())
    }
}
