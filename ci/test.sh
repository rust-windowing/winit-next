#!/bin/sh
# MIT/Apache2 License

set -eu

rx() {
  echo >&2 "+ $*"
  "$@"
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
  rx "${CARGO:-cargo}" run --release -p winit-test-runner -- "$@"
}

basedir="$(dirname -- "$(dirname -- "$0")")"
cd "$basedir" || exit 1

config_path="${1:-basedir/ci/tests_linux.json}"

# Tell which level of test we're running.
case "${2:-2}" in
  0) info "running level 0 (style) tests"; level=0 ;;
  1) info "running level 1 (function) tests"; level=1 ;;
  2) info "running level 2 (full) tests"; level=2 ;;
  *) bail "unknown test level $1" ;;
esac

# Always run style tests.
#test_runner style --config "$config_path"

# At level 1 or higher, run functionality tests.
#if [ "$level" -ge 1 ]; then
#  test_runner functionality --config "$config_path"
#fi

# At level 2 or higher, run full tests.
if [ "$level" -ge 2 ]; then
  test_runner full --config "$config_path"
fi
