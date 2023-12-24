// MIT/Apache2 License

use super::Reporter;
use crate::TestEvent;

use async_net::TcpStream;
use futures_lite::prelude::*;

use std::io;
use std::time::Duration;

/// A reporter that runs over a TCP stream.
pub struct TcpReporter {
    /// TCP stream to send data over.
    socket: TcpStream,
}

impl TcpReporter {
    /// Connect to the given address.
    #[inline]
    pub async fn connect(
        addr: impl async_net::AsyncToSocketAddrs,
        timeout: Duration,
    ) -> io::Result<Self> {
        let result = async move { TcpStream::connect(addr).await };
        let timeout = async move {
            async_io::Timer::after(timeout).await;
            Err(io::ErrorKind::TimedOut.into())
        };

        let socket = result.or(timeout).await?;
        Ok(Self { socket })
    }
}

impl Reporter for TcpReporter {
    fn report(
        &mut self,
        test: TestEvent,
    ) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
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
