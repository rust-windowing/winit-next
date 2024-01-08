// MIT/Apache2 License

//! A testing framework designed to be used internally in `winit`.

pub mod reporter;

use async_channel::Sender;
use async_lock::Mutex;
use futures_lite::{future, prelude::*};
use owo_colors::OwoColorize;
use reporter::Reporter;
use serde::{Deserialize, Serialize};
use web_time::Duration;

use std::borrow::Cow;
use std::env;
use std::future::Future;
use std::io;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};

const DEFAULT_TCP_CONNECT_TIMEOUT: u64 = 15;

/// The event of a test.
#[derive(Serialize, Deserialize, Debug, Clone)]
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestResult {
    /// The name of the test.
    pub name: Cow<'static, str>,

    /// The status of the test.
    pub status: TestStatus,

    /// Description of the test failure.
    pub failure: Cow<'static, str>,
}

/// The status of the test.
#[derive(Serialize, Deserialize, Debug, Clone)]
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
#[allow(clippy::never_loop)]
pub fn run_tests<T>(f: impl FnOnce(&TestHarness) -> T) -> T {
    // Set up hooks.
    tracing_subscriber::fmt::try_init().ok();
    color_eyre::install().ok();

    // Figure out which reporter we're using.
    let reporter: Box<dyn reporter::Reporter + Send> =
        if let Ok(address) = env::var("winit_TEST_TCP_ADDRESS") {
            Box::new(
                future::block_on(reporter::StreamReporter::connect(
                    async_net::TcpStream::connect(address),
                    Duration::from_secs(
                        env::var("winit_TEST_TCP_TIMEOUT")
                            .ok()
                            .and_then(|timeout| timeout.parse::<u64>().ok())
                            .unwrap_or(DEFAULT_TCP_CONNECT_TIMEOUT),
                    ),
                ))
                .expect("failed to connect to TCP port"),
            )
        } else {
            loop {
                #[cfg(unix)]
                {
                    if let Ok(path) = env::var("winit_TEST_UDS_SOCKET") {
                        break Box::new(
                            future::block_on(reporter::StreamReporter::connect(
                                async_net::unix::UnixStream::connect(path),
                                Duration::from_secs(
                                    env::var("winit_TEST_UDS_TIMEOUT")
                                        .ok()
                                        .and_then(|timeout| timeout.parse::<u64>().ok())
                                        .unwrap_or(DEFAULT_TCP_CONNECT_TIMEOUT),
                                ),
                            ))
                            .expect("failed to connect to Unix socket"),
                        );
                    }
                }

                if cfg!(target_os = "android") {
                    break Box::new(reporter::DumpReporter::new());
                }

                // By default, use the console reporter.
                break Box::new(reporter::ConsoleReporter::new());
            }
        };

    // Create our test harness.
    let harness = TestHarness {
        reporter: Mutex::new(reporter),
        count: AtomicUsize::new(0),
    };

    // Run the tests.
    let value = f(&harness);

    // Count tests.
    let TestHarness { reporter, count } = harness;
    let mut reporter = reporter.into_inner();
    future::block_on(reporter.report(TestEvent::End {
        count: count.into_inner(),
    }));

    // Finish with an exit code.
    let code = reporter.finish();
    std::process::exit(code);

    value
}

/// Drive a TCP listener at the specified port.
pub async fn run_tcp_listener(
    port: u16,
    reporter: impl Reporter + Send + 'static,
    once_ready: Sender<()>,
) -> io::Result<()> {
    // Bind to a listening port.
    let listener =
        async_net::TcpListener::bind((async_net::IpAddr::from([0u8, 0, 0, 0]), port)).await?;

    // Wait for the client to connect.
    println!(
        "{} {:?}{}",
        "listening at".white().italic(),
        listener.local_addr()?.cyan().bold(),
        ", waiting for connection...".white().italic()
    );
    once_ready.send(()).await.ok();
    let (socket, addr) = async { listener.accept().await }
        .or(async {
            // Five-minute timeout.
            async_io::Timer::after(Duration::from_secs(60 * 5)).await;
            Err(io::ErrorKind::TimedOut.into())
        })
        .await?;
    drop(listener);

    println!(
        "{}{:?}",
        "got connection at address ".white().italic(),
        addr.cyan().bold()
    );

    run_over_stream(socket, reporter).await
}

/// Drive a Unix listener at the specified path.
#[cfg(unix)]
pub async fn run_unix_listener(
    path: &std::path::Path,
    reporter: impl Reporter + Send + 'static,
    once_ready: Sender<()>,
) -> io::Result<()> {
    use async_net::unix::UnixListener;

    // Bind to a listening port.
    let listener = UnixListener::bind(path)?;

    // Wait for the client to connect.
    println!(
        "{} {:?}{}",
        "listening at".white().italic(),
        listener.local_addr()?.cyan().bold(),
        ", waiting for connection...".white().italic()
    );
    once_ready.send(()).await.ok();
    let (socket, addr) = async { listener.accept().await }
        .or(async {
            // Five-minute timeout.
            async_io::Timer::after(Duration::from_secs(60 * 5)).await;
            Err(io::ErrorKind::TimedOut.into())
        })
        .await?;
    drop(listener);

    println!(
        "{}{:?}",
        "got connection at address ".white().italic(),
        addr.cyan().bold()
    );

    run_over_stream(socket, reporter).await
}

#[inline]
async fn run_over_stream(
    mut socket: impl futures_lite::AsyncRead + Send + Unpin,
    reporter: impl Reporter + Send + 'static,
) -> io::Result<()> {
    // Start reading from the socket.
    let mut buf = Vec::with_capacity(4096);
    let reporter = Mutex::new(reporter);
    let ex = async_executor::Executor::new();
    let mut handles = vec![];

    ex.run({
        let ex = &ex;
        let reporter = &reporter;
        async move {
            loop {
                let mut bytes_to_read = [0u8; 8];

                // Read number of bytes to read from the stream.
                match socket.read_exact(&mut bytes_to_read).await {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                }

                // Read the remaining bytes in this packet.
                buf.resize(u64::from_le_bytes(bytes_to_read) as usize, 0);
                socket.read_exact(&mut buf).await?;

                // Parse to JSON.
                let event: TestEvent = serde_json::from_slice(&buf).expect("failed to parse JSON");

                // Spawn a task to write the event to the reporter.
                handles.push(ex.spawn(async move {
                    let mut reporter = reporter.lock().await;
                    reporter.report(event).await;
                }));
            }

            // Wait for all of the tasks to finish.
            for handle in handles {
                handle.await;
            }

            Ok(())
        }
    })
    .await
}
