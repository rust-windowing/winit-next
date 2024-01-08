// MIT/Apache2 License

/// Implements the `Reporter` trait around something that can asynchronously write.
use super::Reporter;
use crate::{TestEvent, TestResult, TestStatus};

use futures_lite::prelude::*;

use std::io;
use std::time::Duration;

/// A reporter that runs over a stream.
pub struct StreamReporter<S> {
    /// Current exit code.
    exit_code: i32,

    /// TCP stream to send data over.
    socket: S,
}

impl<S> StreamReporter<S> {
    /// Connect to the given address.
    #[inline]
    pub async fn connect(
        stream: impl Future<Output = io::Result<S>>,
        timeout: Duration,
    ) -> io::Result<Self> {
        let timeout = async move {
            async_io::Timer::after(timeout).await;
            Err(io::ErrorKind::TimedOut.into())
        };

        let socket = stream.or(timeout).await?;
        Ok(Self {
            socket,
            exit_code: 0,
        })
    }
}

impl<S: AsyncWrite + Send + Unpin> Reporter for StreamReporter<S> {
    fn report(
        &mut self,
        test: TestEvent,
    ) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            if let TestEvent::Result(TestResult {
                status: TestStatus::Failed,
                ..
            }) = &test
            {
                self.exit_code = 1;
            }

            // The format is the JSON bytes preceded by the number of bytes expected.
            let mut bytes = serde_json::to_vec(&test).expect("failed to serialize TestEvent");
            let count = bytes.len() as u64;
            bytes.splice(0..0, count.to_le_bytes());

            // Write these bytes to the stream.
            self.socket
                .write_all(&bytes)
                .await
                .expect("failed to write to other end of stream");
        })
    }

    fn finish(&mut self) -> i32 {
        0
    }
}
