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

docker build . --tag $IMAGE:$VERSION --tag $IMAGE:latest
docker push $IMAGE:$VERSION
docker push $IMAGE:latest

scp $REPO/SERVER.env admin@$SERVER_IP:/home/admin/app/.env

ssh admin@$SERVER_IP "bash -s $VERSION" < $SCRIPTS/upgrade_image_on_server.sh
