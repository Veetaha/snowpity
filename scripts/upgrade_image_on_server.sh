#!/usr/bin/env bash

set -eu -o pipefail

if [ -z "$1" ]; then
    echo "No argument supplied, this script expects a docker tag to run."
    exit 1
fi

TAG=$1

GIT_REPO_NAME=veebot-telegram
GIT_REMOTE_REPO_URL="https://github.com/Veetaha/$GIT_REPO_NAME.git"
GIT_LOCAL_REPO="$HOME/app/$GIT_REPO_NAME"

if [ ! -d "$GIT_LOCAL_REPO/.git" ]
then
    echo "Cloning repo"

    git clone $GIT_REMOTE_REPO_URL $GIT_LOCAL_REPO
    cd $GIT_LOCAL_REPO
else
    echo "Pulling repo"

    cd $GIT_LOCAL_REPO
    git pull origin master --ff-only
fi

# Copy the `.env` file config that was previously copied into the repo dir
cp ../.env .env

IMAGE="veetaha/veebot-telegram"

echo "Starting deployment for docker image $IMAGE:$TAG"

echo "Removing containers, volume and networks older than 1 week..."

docker system prune --force --filter "until=168h"

echo "Pulling image $IMAGE:$TAG"

docker pull $IMAGE:$TAG

echo "[Re]starting containers..."
CURRENT_UID=$(id -u):$(id -g) docker compose up --detach --no-build

echo "Deployment done"
