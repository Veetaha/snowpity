#!/usr/bin/env bash

set -euo pipefail

release_flag=""

if [[ $RUST_BUILD_MODE == "release" ]]; then
    release_flag=--release
fi

SQLX_OFFLINE=true

cargo build \
    $release_flag \
    --target-dir /target \
    --package snowpity-tg \
    --bin snowpity-tg

# The buildkit's cache dir (`/target`) isn't part of docker layers, so we need to
# copy the binary out of that dir into somewhere where it will be part of the layer.
cp /target/$RUST_BUILD_MODE/snowpity-tg /usr/bin/
