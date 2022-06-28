#!/usr/bin/env bash

set -eu -o pipefail

if [ -z "$1" ]; then
    echo "No argument supplied, this script expects a docker tag to run."
    exit 1
fi

tag=$1

git_repo_name=veebot-telegram
git_remote_repo_url="https://github.com/Veetaha/$git_repo_name.git"
git_local_repo="$HOME/app/$git_repo_name"

if [ ! -d "$git_local_repo/.git" ]
then
    echo "Cloning repo"

    git clone $git_remote_repo_url $git_local_repo
    cd $git_local_repo
else
    echo "Pulling repo"

    cd $git_local_repo
    git pull origin master --ff-only
fi

# Copy the `.env` file config that was previously copied into the repo dir
cp ../.env .env

image="veetaha/veebot-telegram"

echo "Starting deployment for docker image $image:$tag"

echo "Removing containers, volume and networks older than 1 week..."

docker system prune --force --filter "until=168h"

echo "Pulling image $image:$tag"

docker pull $image:$tag

echo "[Re]starting containers..."
current_uid=$(id -u):$(id -g) docker compose up --detach --no-build

echo "Deployment done"
