#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.16.11

base_url="https://github.com/rust-cross/cargo-zigbuild/releases/download/v$version"

file_stem="cargo-zigbuild-v$version.$arch_rust-unknown-linux-musl"

curl_and_decompress $base_url/$file_stem.tar.gz

zig_version=0.10.1

curl_and_decompress \
    https://ziglang.org/download/$zig_version/zig-linux-$arch_rust-$zig_version.tar.xz \
    -C /usr/local

ln -s /usr/local/zig-linux-$arch_rust-$zig_version/zig /usr/local/bin/zig
