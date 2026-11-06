#!/usr/bin/env bash
# Check that the server finished its setup and runs every container of the bot
# on the expected images. The first argument is the image tag of the bot.
set -euo pipefail

tag=$1

on_error() {
    echo "The check of the server failed. The end of the cloud-init log:" >&2
    sudo tail --lines 50 /var/log/cloud-init-output.log >&2
}
trap on_error ERR

# Exit code 2 is a recoverable error, such as a deprecated config key.
# https://cloudinit.readthedocs.io/en/latest/explanation/return_codes.html
status=0
cloud-init status --wait --long || status=$?
if ((status != 0 && status != 2)); then
    false
fi

systemctl is-active tg-bot.service

mapfile -t containers < <(docker ps --all --quiet)

failed=0
while read -r name image state restarts; do
    echo "$name $image $state restarts=$restarts"
    if [[ $state != running || $restarts != 0 ]]; then
        echo "The container $name is not healthy" >&2
        failed=1
    fi
    if [[ $image == *snowpity-tg* && $image != *:$tag && $image != *:grafana-$tag ]]; then
        echo "The container $name runs an image of another commit" >&2
        failed=1
    fi
done < <(
    docker inspect \
        --format '{{.Name}} {{.Config.Image}} {{.State.Status}} {{.RestartCount}}' \
        "${containers[@]}"
)

((failed == 0))
