# winit-test-runner

Test runner for `winit`.

This binary acts as orchestration for running tests in `winit`. In addition to
what is often used in normal Rust tests (formatting, Clippy, unit tests), it also
runs those tests in cross-compilation environments on other platforms.

There are different types of tests:

- **Style tests** make sure that code is formatted and linted properly.
  `cargo fmt` and `cargo clippy` are used to inspect Rust code.
- **Functionality tests** run the doctests and unit tests in Rust code. These
  often ensure that basic functionality and logic are in working order.
- **Host tests** run the `winit` test suite on the current host. This test suite
  fully tests the functionality of `winit` to ensure that it is working properly.
  A full CI run with `winit` should be fully bug-free.
- **Cross tests** run the `winit` test suite in Docker containers/virtual
  machines in order to ensure `winit` works on all possible hosts.

## License

MIT/Apache2
