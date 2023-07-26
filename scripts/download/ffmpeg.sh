#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

. $script_dir/common.sh

# Download from https://johnvansickle.com/ffmpeg/
# The URL format is not documented, it was just inferred from the links on the page

version=6.0

file_stem=ffmpeg-$version-$arch_go-static

url=https://johnvansickle.com/ffmpeg/releases/$file_stem.tar.xz

# FIXME: use `md5sum` to verify the integrity of the downloaded file
# See https://www.johnvansickle.com/ffmpeg/faq/
curl_and_decompress $url --strip-components 1 $file_stem/ffmpeg
