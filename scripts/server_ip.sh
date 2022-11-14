#!/usr/bin/env bash

scripts=$(readlink -f $(dirname $0))
repo="$scripts/.."

export server_ip=$(cd $repo/deployment/project && terraform output -json | jq -r '.server.value.ipv6')

echo "Server IP: $server_ip"
