#!/usr/bin/env bash

set -euo pipefail

sudo chown -R $(id --user):$(id --group) $DATA_VOLUME_MOUNT_POINT

mkdir -p $PG_DATA

/var/app/docker-compose.sh up --detach --no-build --wait
