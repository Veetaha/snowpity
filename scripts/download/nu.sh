#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.81.0

base_url="https://github.com/nushell/nushell/releases/download/$version"

curl_and_decompress $base_url/nu-$version-x86_64-unknown-linux-musl.tar.gz \
  nu-$version-x86_64-unknown-linux-musl/nu nu-$version-x86_64-unknown-linux-musl/nu_plugin_formats --strip-components 1
