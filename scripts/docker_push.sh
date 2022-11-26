#!/usr/bin/env bash

# TODO: rewrite this in nushell

set -eu -o pipefail

scripts=$(readlink -f $(dirname $0))

. scripts/docker_push.

docker push $image:$version
docker push $image:latest
