#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.72.0

base_url="https://github.com/nushell/nushell/releases/download/$version"

curl_tar_gz $base_url/nu-$version-x86_64-unknown-linux-musl.tar.gz \
  nu-$version-x86_64-unknown-linux-musl/nu --strip-components 1
