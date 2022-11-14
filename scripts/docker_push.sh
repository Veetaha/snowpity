#!/usr/bin/env bash

# TODO: rewrite this in nushell

set -eu -o pipefail

scripts=$(readlink -f $(dirname $0))
repo="$scripts/.."

cd $repo

image=$(cd deployment/project && terraform output -json | jq -r '.docker.value.image_name')

version=$(\
    cargo metadata --format-version=1 \
    | jq -r '.packages[] | select(.name == "veebot-telegram") | .version' \
)

workspace=$(cd deployment/project && terraform workspace show)

if [ "$workspace" = "default" ]; then
    build_mode=release
else
    build_mode=debug
fi

echo "Building docker image $image:$version in $build_mode mode"

DOCKER_BUILDKIT=1 docker build . --tag $image:$version --tag $image:latest --build-arg RUST_BUILD_MODE=$build_mode
docker push $image:$version
docker push $image:latest
