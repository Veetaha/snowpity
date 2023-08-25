#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.11.0

base_url="https://github.com/koute/bytehound/releases/download/$version"

curl_and_decompress $base_url/bytehound-$arch_rust-unknown-linux-gnu.tgz bytehound libbytehound.so
