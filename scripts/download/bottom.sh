#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

version=0.9.3

deb_file=bottom_${version}_$arch_go.deb

# FIXME: use retries
curl -LO https://github.com/ClementTsang/bottom/releases/download/$version/$deb_file
sudo dpkg -i $deb_file
rm $deb_file
