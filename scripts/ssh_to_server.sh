#!/usr/bin/env bash

SCRIPTS=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

. $SCRIPTS/server_ip.sh

ssh admin@$SERVER_IP
