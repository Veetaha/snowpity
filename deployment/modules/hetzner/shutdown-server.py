#!/usr/bin/env python3
"""
Shut down a Hetzner Cloud server gracefully and wait until it is off.

The server ID is the first argument. The API token is in `HCLOUD_TOKEN`.

The OS gets an ACPI shutdown request. `systemd` then stops the services in
order, so the database flushes its data before the data volume is unmounted.
"""

import json
import os
import sys
import time
import urllib.request
from typing import TypedDict, cast

API = "https://api.hetzner.cloud/v1"
TIMEOUT_SECS = 300
POLL_SECS = 5


class Server(TypedDict):
    status: str


class ServerResponse(TypedDict):
    server: Server


def request(method: str, path: str) -> object:
    req = urllib.request.Request(
        f"{API}{path}",
        method=method,
        headers={"Authorization": f"Bearer {os.environ['HCLOUD_TOKEN']}"},
    )
    with urllib.request.urlopen(req) as res:
        return json.load(res)


def status(server_id: str) -> str:
    res = cast(ServerResponse, request("GET", f"/servers/{server_id}"))
    return res["server"]["status"]


def main() -> None:
    server_id = sys.argv[1]

    if status(server_id) == "off":
        print(f"Server {server_id} is already off")
        return

    # https://docs.hetzner.cloud/reference/cloud#server-actions-shutdown-a-server
    request("POST", f"/servers/{server_id}/actions/shutdown")
    print(f"Sent the shutdown request to server {server_id}")

    deadline = time.monotonic() + TIMEOUT_SECS
    while time.monotonic() < deadline:
        time.sleep(POLL_SECS)
        if status(server_id) == "off":
            print(f"Server {server_id} is off")
            return

    sys.exit(f"Server {server_id} did not shut down in {TIMEOUT_SECS} seconds")


if __name__ == "__main__":
    main()
