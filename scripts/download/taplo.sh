#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.8.0

base_url="https://github.com/tamasfe/taplo/releases/download/$version"

file_name=taplo-full-linux-$arch_rust

curl_and_decompress $base_url/$file_name.gz

mv $file_name ./taplo
chmod +x ./taplo
