#!/usr/bin/env bash

scripts=$(readlink -f $(dirname $0))

. $scripts/server_ip.sh

ssh -t ubuntu@$server_ip "clear && cat /var/log/cloud-init-output.log"
