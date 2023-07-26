#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=v0.5.0

base_url="https://github.com/bnjbvr/cargo-machete/releases/download/$version"

file_stem="cargo-machete-$version-$arch_rust-unknown-linux-musl"

curl_and_decompress $base_url/$file_stem.tar.gz --strip-components 1 $file_stem/cargo-machete
