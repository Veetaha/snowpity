#!/usr/bin/env bash

# TODO: rewrite this in Rust

set -eu -o pipefail

SCRIPTS=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO="$SCRIPTS/.."

. $SCRIPTS/server_ip.sh

scp $REPO/SERVER.env admin@$SERVER_IP:/home/admin/app/.env

ssh admin@$SERVER_IP "bash -s $VERSION" < $SCRIPTS/on_server/reload.sh
