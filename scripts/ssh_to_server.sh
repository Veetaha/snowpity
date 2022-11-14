#!/usr/bin/env bash

scripts=$(readlink -f $(dirname $0))

. $scripts/server_ip.sh

ssh admin@$server_ip
