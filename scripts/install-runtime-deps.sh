#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

deps=(
    # Not an expert in SSL, but this seems to be required for all SSL-encrypted communication.
    # Thanks to this guy for help:
    # https://github.com/debuerreotype/docker-debian-artifacts/issues/15#issuecomment-634423712
    ca-certificates
    libopus0
)

# Required for downloading the dependencies
tmp_deps=(
    curl
    xz-utils
)

apt-get update
apt-get install --yes --no-install-recommends "${deps[@]}" "${tmp_deps[@]}"

$script_dir/download/ffmpeg.sh
mv ./ffmpeg /usr/bin/ffmpeg

apt-get remove --yes "${tmp_deps[@]}"

rm -rf /var/lib/apt/lists/*
