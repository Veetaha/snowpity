#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.13.7

base_url="https://github.com/EmbarkStudios/cargo-deny/releases/download/$version"

file_stem="cargo-deny-$version-x86_64-unknown-linux-musl"

curl_and_decompress $base_url/$file_stem.tar.gz --strip-components 1 $file_stem/cargo-deny
