#!/usr/bin/env bash

SCRIPTS=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO="$SCRIPTS/.."

export SERVER_IP=$(cd $REPO/deployment/project && terraform output -json | jq -r '.server.value.ipv6')

echo "Server IP: $SERVER_IP"
