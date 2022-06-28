#!/usr/bin/env bash

# TODO: rewrite this in Rust

set -eu -o pipefail

image="veetaha/veebot-telegram"

scripts=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
repo="$scripts/.."

cd $repo

version=$(\
    cargo metadata --format-version=1 \
    | jq -r '.packages[] | select(.name == "veebot-telegram") | .version' \
)

# docker build . --tag $image:$version --tag $image:latest
# docker push $image:$version

server_ip=$(cd $repo/deployment/hetzner && terraform output -json | jq -r '.server_ip.value')

echo "Server IP: $server_ip"

scp $repo/SERVER.env admin@$server_ip:/home/admin/app/.env

ssh admin@$server_ip "bash -s $version" < $scripts/upgrade_image_on_server.sh
