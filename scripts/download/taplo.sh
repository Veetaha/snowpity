#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.7.0

base_url="https://github.com/tamasfe/taplo/releases/download/release-taplo-cli-$version"

curl_tar_gz $base_url/taplo-x86_64-unknown-linux-gnu.tar.gz taplo
