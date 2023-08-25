#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=1.36.0

base_url="https://github.com/hetznercloud/cli/releases/download/v$version"

curl_and_decompress $base_url/hcloud-linux-$arch_go.tar.gz hcloud
