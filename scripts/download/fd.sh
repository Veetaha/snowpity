#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=8.7.0

deb_file=fd_${version}_$arch_go.deb

# FIXME: use retries
curl -LO https://github.com/sharkdp/fd/releases/download/v$version/$deb_file
sudo dpkg -i $deb_file
rm $deb_file
