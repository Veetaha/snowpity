#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.13.1

base_url="https://github.com/EmbarkStudios/cargo-deny/releases/download/$version"

curl_tar_gz \
    $base_url/cargo-deny-$version-x86_64-unknown-linux-musl.tar.gz \
    --strip-components 1 cargo-deny-$version-x86_64-unknown-linux-musl/cargo-deny
