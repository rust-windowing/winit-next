// MIT/Apache2 License

use futures_lite::future::block_on;

#[allow(clippy::eq_op)]
fn main() {
    winit_test::run_tests(|harness| {
        block_on(async move {
            harness.test("hello world", async {
                assert_eq!(1 + 1, 2)
            }).await;
        });
    })
}
