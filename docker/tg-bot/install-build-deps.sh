#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/../../scripts/download/common.sh

golang_version="1.20.5"

# Required for downloading the dependencies
tmp_deps=(
    curl
    xz-utils
)

apt-get update
apt-get install --yes --no-install-recommends "${tmp_deps[@]}"

rm -rf /usr/local/go

curl_and_decompress https://go.dev/dl/go$golang_version.linux-$arch_go.tar.gz -C /usr/local
