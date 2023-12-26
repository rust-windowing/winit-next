#!/bin/sh
# MIT/Apache2 License

set -eu

rx() {
  cmd_rx="$1"
  shift
  (
    set -x
    "$cmd_rx" "$@"
  )
}
no_out() {
  cmd_noout="$1"
  shift
  "$cmd_noout" "$@" > /dev/null 2> /dev/null
}
bail() {
  echo "[fatal] $*"
  exit 1
}
info() {
  echo "[info] $*"
}
bail_if_absent() {
  if ! no_out command -v "$1"; then
    bail "could not find $1"
  fi
}
test_runner() {
  bail_if_absent "${CARGO:-cargo}"
  rx "${CARGO:-cargo}" run --release -p keter-test-runner -- "$@"
}

basedir="$(dirname -- "$(dirname -- "$0")")"
cd "$basedir" || exit 1

config_path="${1:-basedir/ci/tests_linux.json}"

# Tell which level of test we're running.
case "${2:-2}" in
  0) info "running level 0 (style) tests"; level=0 ;;
  1) info "running level 1 (function) tests"; level=1 ;;
  2) info "running level 2 (host) tests"; level=2 ;;
  3) info "running level 3 (cross) tests"; level=3 ;;
  *) bail "unknown test level $1" ;;
esac

# Always run style tests.
test_runner style --config "$config_path"

# At level 1 or higher, run functionality tests.
if [ "$level" -gt 1 ]; then
  test_runner functionality --config "$config_path"
fi
