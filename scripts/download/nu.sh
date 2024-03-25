#!/usr/bin/env bash

set -euxo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.87.1

base_url="https://github.com/nushell/nushell/releases/download/$version"
file_stem="nu-$version-$arch_rust-unknown-linux-gnu"

curl_and_decompress \
    $base_url/$file_stem.tar.gz \
    $file_stem/nu \
    --strip-components 1
