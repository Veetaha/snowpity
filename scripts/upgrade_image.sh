#!/usr/bin/env bash

# TODO: rewrite this in Rust

set -eu -o pipefail

image="veetaha/veebot-telegram"

SCRIPTS=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO="$SCRIPTS/.."

cd $REPO

VERSION=$(\
    cargo metadata --format-version=1 \
    | jq -r '.packages[] | select(.name == "veebot-telegram") | .version' \
)

# docker build . --tag $image:$version --tag $image:latest
# docker push $image:$version

SERVER_IP=$(cd $REPO/deployment/hetzner && terraform output -json | jq -r '.server_ip.value')

echo "Server IP: $SERVER_IP"

scp $REPO/SERVER.env admin@$SERVER_IP:/home/admin/app/.env

ssh admin@$SERVER_IP "bash -s $VERSION" < $SCRIPTS/upgrade_image_on_server.sh
