#!/bin/sh

set -eu

rx() {
  echo >&2 "+ $*"
  "$@"
}
build_dockerfile() {
  tag="$1"
  name="$2"

  rx docker build . -f dockerfiles/"$name" \
      -t ghcr.io/notgull/winit_test:"$tag"
}

build_dockerfile ubuntu Dockerfile.ubuntu
