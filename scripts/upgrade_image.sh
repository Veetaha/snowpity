#!/usr/bin/env bash

set -eu -o pipefail

image="veetaha/veebot-telegram"

scripts=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
repo="$scripts/.."

cd $repo

version=$(\
    cargo metadata --format-version=1 \
    | jq -r '.packages[] | select(.name == "veebot-telegram") | .version' \
)

docker build . --tag $image:$version
docker push $image:$version

server_ip=$(cd $repo/deployment/hetzner && terraform output -json | jq -r '.server_ip.value')

ssh admin@$server_ip "bash -s $1" < $scripts/upgrade_image_on_server.sh
