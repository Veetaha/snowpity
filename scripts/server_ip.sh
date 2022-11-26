#!/usr/bin/env nu

scripts=$(readlink -f $(dirname $0))
repo="$scripts/.."

export server_ip=$(cd $repo/deployment/project && terraform output -json | jq -r '.server.value.ip')

echo "Server IP: $server_ip"
