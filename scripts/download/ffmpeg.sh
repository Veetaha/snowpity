#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

# Download from https://johnvansickle.com/ffmpeg/
# The URL format is not documented, it was just inferred from the links on the page

version=6.0

file_stem=ffmpeg-$version-amd64-static

url=https://johnvansickle.com/ffmpeg/releases/$file_stem.tar.xz

curl_and_decompress $url --strip-components 1 $file_stem/ffmpeg
