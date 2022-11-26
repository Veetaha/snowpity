#!/usr/bin/env bash

echo "Running: docker compose $@"

CURRENT_UID=$(id -u):$(id -g) docker compose $@
