#!/usr/bin/env bash

set -euo pipefail

script_dir=$(readlink -f $(dirname $0))

deps=(
    # TLS requires the trusted root certificates. They are not present by default in our
    # debian image that we in docker. This was also motivated by the following issue:
    # https://github.com/debuerreotype/docker-debian-artifacts/issues/15#issuecomment-634423712
    ca-certificates
    # Required for ogg_opus crate (via auidopus_sys). Even though we could link it
    # statically, it's easier to just install it.
    libopus0
)

# Required for downloading the dependencies
tmp_deps=(
    curl
    xz-utils
)

apt-get update
apt-get install --yes --no-install-recommends "${deps[@]}" "${tmp_deps[@]}"

$script_dir/../../scripts/download/ffmpeg.sh
mv ./ffmpeg /usr/bin/ffmpeg

apt-get remove --yes "${tmp_deps[@]}"

rm -rf /var/lib/apt/lists/*
