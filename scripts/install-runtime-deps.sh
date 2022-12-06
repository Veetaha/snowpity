#!/usr/bin/env bash

set -euo pipefail

deps=(
    # Not an expert in SSL, but this seems to be required for all SSL-encrypted communication.
    # Thanks to this guy for help:
    # https://github.com/debuerreotype/docker-debian-artifacts/issues/15#issuecomment-634423712
    ca-certificates

    ffmpeg

    libopus0
)

apt-get update
apt-get install -y --no-install-recommends "${deps[@]}"
rm -rf /var/lib/apt/lists/*
