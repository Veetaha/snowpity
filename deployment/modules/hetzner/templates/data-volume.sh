#!/usr/bin/env bash

set -euo pipefail

sudo chown -R $(id --user):$(id --group) $DATA_VOLUME_PATH
mkdir -p $PG_DATA

mkdir -p $DATA_VOLUME_PATH/grafana-agent-wal
sudo chown -R \
    $(id grafana-agent --user):$(id grafana-agent --group) \
    $DATA_VOLUME_PATH/grafana-agent-wal
