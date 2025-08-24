#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=v1.35.3

base_url="https://github.com/crate-ci/typos/releases/download/$version"

file_stem="typos-$version-$arch_rust-unknown-linux-musl"

curl_and_decompress $base_url/$file_stem.tar.gz ./typos
