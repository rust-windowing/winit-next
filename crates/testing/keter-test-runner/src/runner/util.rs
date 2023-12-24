// MIT/Apache2 License

use async_executor::{Executor, Task};
use once_cell::sync::OnceCell;

use std::future::{pending, Future};
use std::thread;

/// Spawn a future onto the global executor.
pub(crate) fn spawn<F: Future + Send + 'static>(f: F) -> Task<F::Output>
where
    F::Output: Send + 'static,
{
    executor().spawn(f)
}

/// Run the executor alongside this future.
pub(crate) async fn run<F: Future>(f: F) -> F::Output {
    executor().run(f).await
}

fn executor() -> &'static Executor<'static> {
    static EXECUTOR: OnceCell<Executor<'static>> = OnceCell::new();

    EXECUTOR.get_or_init(|| {
        // Only use two executor threads.
        for i in 0..4 {
            thread::Builder::new()
                .name(format!("keter-test-runner-{i}"))
                .spawn(|| {
                    async_io::block_on(executor().run(pending::<()>()));
                })
                .expect("failed to spawn runner thread");
        }

        Executor::new()
    })
}
