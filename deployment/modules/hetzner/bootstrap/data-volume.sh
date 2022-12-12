#!/usr/bin/env bash

set -euo pipefail

sudo chown -R $(id --user):$(id --group) $DATA_VOLUME_PATH
sudo mkdir -p $DATA_VOLUME_PATH/docker
