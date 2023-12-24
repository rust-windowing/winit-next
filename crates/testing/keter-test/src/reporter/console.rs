// MIT/Apache2

use super::Reporter;
use crate::{TestEvent, TestResult, TestStatus};

use futures_lite::prelude::*;
use owo_colors::OwoColorize;

use std::borrow::Cow;
use std::fmt::{self, Write as _};
use std::future::Future;
use std::io::{self, prelude::*};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Report tests to the output console.
pub struct ConsoleReporter(Arc<Mutex<Inner>>);

struct Inner {
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
        Self(Arc::new(Mutex::new(Inner {
            exit_code: 0,
            indent: 0,
            failures: vec![],
        })))
    }
}

impl Reporter for ConsoleReporter {
    #[inline]
    fn report(&mut self, test: TestEvent) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        let this = self.0.clone();
        blocking::unblock(move || {
            let mut this = this.lock().unwrap();
            let mut cout = io::stdout().lock();

            match test {
                TestEvent::End { count: _ } => {
                    if this.exit_code == 0 {
                        writeln!(cout, "{}{}", "test result: ".white(), "ok".green()).unwrap();
                    } else {
                        writeln!(cout, "{}{}", "test result: ".white(), "FAILED".red()).unwrap();
                    }
                }
                TestEvent::BeginGroup { name, count } => {
                    writeln!(
                        cout,
                        "{}{}{}{}{}{}",
                        Indent(this.indent),
                        "running test group '".white().italic(),
                        name.cyan().bold(),
                        "' with ".white().italic(),
                        count.cyan().bold(),
                        " tests...".white().italic()
                    )
                    .unwrap();

                    this.indent += 1;
                }
                TestEvent::EndGroup(_name) => {
                    this.indent -= 1;
                }
                TestEvent::Result(TestResult {
                    name,
                    status,
                    failure,
                }) => {
                    write!(
                        cout,
                        "{}{}{}{}",
                        Indent(this.indent),
                        "test ".white(),
                        name.bold().white(),
                        "... ".white()
                    )
                    .unwrap();

                    match status {
                        TestStatus::Failed => {
                            this.failures.push((name, failure));
                            this.exit_code = 1;
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
        })
        .boxed()
    }

    #[inline]
    fn finish(&mut self) -> i32 {
        self.0.lock().unwrap().exit_code
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
