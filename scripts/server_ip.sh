#!/usr/bin/env bash

SCRIPTS=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO="$SCRIPTS/.."

export SERVER_IP=$(cd $REPO/deployment/prod && terraform output -json | jq -r '.server.value.ip')

echo "Server IP: $SERVER_IP"
