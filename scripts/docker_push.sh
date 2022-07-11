#!/usr/bin/env bash

# TODO: rewrite this in Rust

set -eu -o pipefail

IMAGE="veetaha/veebot-telegram"

SCRIPTS=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO="$SCRIPTS/.."

. $SCRIPTS/server_ip.sh

cd $REPO

VERSION=$(\
    cargo metadata --format-version=1 \
    | jq -r '.packages[] | select(.name == "veebot-telegram") | .version' \
)

docker build . --tag $IMAGE:$VERSION --tag $IMAGE:latest --build-arg RUST_BUILD_MODE=release
docker push $IMAGE:$VERSION
docker push $IMAGE:latest
