#!/usr/bin/env python3
"""A dead-simple CLI that automates development processes in this repository."""

import argparse
import functools
import json
import os
import random
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent
TF_PROJECT = REPO / "deployment" / "project"

#####################################
############ Entrypoints ############
#####################################


def cmd_ssh(args):
    if args.code:
        # FIXME: make vscode connection work via CLI, it doesn't right now :(
        run("code", "--folder-uri", f"vscode-remote://ssh-remote+{ssh_str()}")
        return
    ssh(forward_ports=args.forward_ports)


def cmd_ssh_str(args):
    print(ssh_str())


def cmd_ssh_journalctl(args):
    ssh("sudo journalctl --follow --catalog --unit tg-bot.service")


def cmd_ssh_systemctl_status(args):
    ssh("sudo systemctl status tg-bot.service")


def cmd_ssh_cloud_init_log(args):
    ssh_cloud_init_log(dump=args.dump)


def cmd_ssh_cloud_init_status(args):
    ssh("cloud-init status")


def cmd_ssh_cloud_init_user_data(args):
    ssh("sudo cloud-init query userdata")


def cmd_docker_build(args):
    docker_build_all(release=args.release, push=args.push)


def cmd_up(args):
    if args.fresh:
        info("--fresh was specified, so deleting the data volumes...")
        down(drop_data=True)

    up_args = ["up", "--remove-orphans", "--build", "--wait", "postgres", "pgadmin"]
    if not args.no_tg_bot:
        up_args.append("tg-bot")
    if not args.no_observability:
        up_args += ["victoria-metrics", "grafana", "loki", "grafana-agent"]

    docker_compose(*up_args, env={"RUST_BUILD_MODE": build_mode(args.release)})

    if args.no_tg_bot:
        run("sqlx", "migrate", "run", "--source", "crates/snowpity-tg/migrations", cwd=REPO)

    logs_args = ["logs", "--follow"]
    if args.no_tg_bot:
        logs_args += ["pgadmin", "postgres"]
    else:
        logs_args += ["--no-log-prefix", "tg-bot"]

    docker_compose(*logs_args)


def cmd_down(args):
    down(drop_data=args.drop_data)


def cmd_deploy(args):
    if not args.no_build:
        docker_build_all(release=args.release, push=True)

    tf_args = ["plan" if args.plan else "apply", *tf_vars()]
    if args.yes:
        tf_args.append("--auto-approve")
    if args.drop_server:
        tf_args.append("--replace=module.hetzner.hcloud_server.master")
    if args.drop_data:
        tf_args.append("--replace=module.hetzner.hcloud_volume.master")
    if args.no_tf_refresh:
        tf_args.append("--refresh=false")

    tf(*tf_args)

    if not args.plan:
        # The server boots for a while, and ssh fails until it is up.
        with_retry(lambda: ssh_cloud_init_log(dump=False), max_retries=20)


def cmd_tag(args):
    run("git", "tag", f"v{project_version()}", cwd=REPO)
    run("git", "push", "--tags", cwd=REPO)


def cmd_destroy(args):
    tf_args = ["destroy", *tf_vars()]
    if args.yes:
        tf_args.append("--auto-approve")
    if not args.all:
        tf_args.append("--target=module.hetzner.hcloud_server.master")
        if args.drop_data:
            tf_args.append("--target=module.hetzner.hcloud_volume.master")
    tf(*tf_args)


def cmd_tf_state_list(args):
    tf("state", "list", debug=False)


def cmd_tf_output(args):
    print_json(tf_output())


def cmd_derpibooru_image(args):
    image = http_get_json(f"https://derpibooru.org/api/v1/json/images/{args.id}")["image"]
    # Show the image URLs of each size next to the other fields.
    image.update(image.pop("representations"))
    print_json(image)


def cmd_tg_chat_id(args):
    query = urllib.parse.urlencode({"chat_id": f"@{args.chat_tag}", "text": "snowpity"})
    url = f"https://api.telegram.org/bot{args.bot_token}/sendMessage?{query}"
    print(http_get_json(url)["result"]["chat"]["id"])


def cmd_sqlx_prepare(args):
    run("cargo", "sqlx", "prepare", cwd=REPO / "crates" / "snowpity-tg")


def cmd_test(args):
    run("cargo", "test", "--", *args.args, cwd=REPO, env={"RUST_LOG": "debug,h2=info,hyper=info"})


def cli():
    parser = argparse.ArgumentParser(prog="x", description=__doc__)
    commands = parser.add_subparsers(metavar="<command>", required=True)

    def command(parent, name, func, help):
        sub = parent.add_parser(name, help=help, description=help)
        sub.set_defaults(func=func)
        return sub

    def group(parent, name, help):
        sub = parent.add_parser(name, help=help, description=help)
        return sub.add_subparsers(metavar="<command>", required=True)

    ssh_cmd = command(commands, "ssh", cmd_ssh, "SSH into the app's server")
    ssh_cmd.add_argument(
        "-f", "--forward-ports", action="store_true", help="Forward grafana and pgadmin4 ports to localhost"
    )
    ssh_cmd.add_argument("--code", action="store_true", help="Connect using VSCode")
    ssh_sub = ssh_cmd.add_subparsers(metavar="<command>")
    command(ssh_sub, "str", cmd_ssh_str, "Print the user@ip string for the app's server")
    command(ssh_sub, "journalctl", cmd_ssh_journalctl, "Follow the app's systemd service logs")
    systemctl = ssh_sub.add_parser("systemctl", help="Inspect the app's systemd service")
    systemctl_sub = systemctl.add_subparsers(metavar="<command>", required=True)
    command(systemctl_sub, "status", cmd_ssh_systemctl_status, "Display the app's systemd service status")
    cloud_init = ssh_sub.add_parser("cloud-init", help="Inspect the server's cloud-init")
    cloud_init_sub = cloud_init.add_subparsers(metavar="<command>", required=True)
    log = command(cloud_init_sub, "log", cmd_ssh_cloud_init_log, "Display the server's cloud-init logs")
    log.add_argument(
        "--dump", action="store_true", help="Don't show the tail of the log file, but dump its full contents"
    )
    command(cloud_init_sub, "status", cmd_ssh_cloud_init_status, "Display the server's cloud-init status")
    command(
        cloud_init_sub, "user-data", cmd_ssh_cloud_init_user_data, "Show the ultimate user data residing on the server"
    )

    docker = group(commands, "docker", "Docker images")
    build = command(docker, "build", cmd_docker_build, "Build all docker images")
    build.add_argument("-r", "--release", action="store_true", help="Build in release mode")
    build.add_argument("-p", "--push", action="store_true", help="Push the image to remote docker registry")

    up = command(commands, "up", cmd_up, "Start all services locally using `docker compose`")
    up.add_argument("--no-tg-bot", action="store_true", help="Don't start the tg_bot service")
    up.add_argument(
        "--no-observability", action="store_true", help="Don't start the pgadmin and observability services"
    )
    up.add_argument("-f", "--fresh", action="store_true", help="Delete the data volumes before starting the database")
    up.add_argument("-r", "--release", action="store_true", help="Build in release mode")

    down_cmd = command(
        commands, "down", cmd_down, "Shutdown the local containers and clean the persistent data volumes"
    )
    down_cmd.add_argument("--drop-data", action="store_true", help="Remove all data volumes")

    deploy = command(commands, "deploy", cmd_deploy, "Deploy the full application's stack")
    deploy.add_argument(
        "--no-build",
        action="store_true",
        help="Skip build step, reuse the docker image that is already in the remote registry",
    )
    deploy.add_argument("-r", "--release", action="store_true", help="Build in release mode")
    deploy.add_argument("--drop-server", action="store_true", help="Force the re-creation of the server instance")
    deploy.add_argument(
        "--drop-data", action="store_true", help="Drop the persistent data (re-create the data volume)"
    )
    deploy.add_argument("--plan", action="store_true", help="Do `tf plan` instead of `tf apply`")
    deploy.add_argument("-y", "--yes", action="store_true", help="Auto-approve the deployment")
    deploy.add_argument(
        "--no-tf-refresh", action="store_true", help="Don't refresh the terraform state before deployment"
    )

    command(
        commands, "tag", cmd_tag, "Create a git tag with the current project's version and push it to the remote"
    )

    destroy = command(
        commands,
        "destroy",
        cmd_destroy,
        "Destroy the application's stack. By default destroys only the server instance, "
        "because it's safe to do, and no data will be lost. Use `--all` to destroy everything.",
    )
    destroy.add_argument("-y", "--yes", action="store_true", help="Auto-approve the destruction")
    destroy.add_argument(
        "--drop-data", action="store_true", help="Destroy the Hetzner Cloud volume. ⚠️ This guarantees data loss"
    )
    destroy.add_argument(
        "--all",
        action="store_true",
        help="Destroy all resources. ⚠️ This guarantees data loss because "
        "the database's data volume will be destroyed as well",
    )

    tf_group = group(commands, "tf", "Terraform shortcuts")
    tf_state = tf_group.add_parser("state", help="Terraform state")
    tf_state_sub = tf_state.add_subparsers(metavar="<command>", required=True)
    command(tf_state_sub, "list", cmd_tf_state_list, "Convenience wrapper for `tf state list`")
    command(tf_group, "output", cmd_tf_output, "Convenience wrapper for `tf output`")

    derpibooru = group(commands, "derpibooru", "Derpibooru API")
    image = command(derpibooru, "image", cmd_derpibooru_image, "Fetch the image metadata from derpibooru")
    image.add_argument("id", type=int)

    tg = group(commands, "tg", "Telegram API")
    chat_id = command(
        tg,
        "chat-id",
        cmd_tg_chat_id,
        "Get the ID of the telegram chat by its public tag. "
        "This is needed to get the ID of the private telegram chat or channel. "
        "We have to make that chat temporarily public, then get its ID and move it back to private again.",
    )
    chat_id.add_argument("--bot-token", required=True)
    chat_id.add_argument("--chat-tag", required=True)

    sqlx = group(commands, "sqlx", "sqlx shortcuts")
    command(sqlx, "prepare", cmd_sqlx_prepare, "Run `cargo sqlx prepare` for the bot crate")

    test = command(commands, "test", cmd_test, "Invoke `cargo test` with logging enabled")
    test.add_argument("args", nargs=argparse.REMAINDER, help="Arguments for the test binary")

    return parser


def main():
    args = cli().parse_args()
    try:
        args.func(args)
    except CommandError as err:
        error(err)
        sys.exit(err.exit_code)
    except KeyboardInterrupt:
        sys.exit(130)


################################################
############ Implementation details ############
################################################


class CommandError(Exception):
    def __init__(self, invocation, exit_code):
        super().__init__(f"Command exited with code {exit_code}\n{invocation}")
        self.exit_code = exit_code


def run(*cmd, cwd=None, env=None, debug=True, capture=False):
    """Run a command and raise `CommandError` when it fails.

    Return its stdout when `capture` is set.
    """
    invocation = " ".join(str(arg) for arg in cmd)
    if debug:
        log("34", "DEBUG", invocation)
    result = subprocess.run(
        [str(arg) for arg in cmd],
        cwd=cwd,
        env={**os.environ, **env} if env else None,
        stdout=subprocess.PIPE if capture else None,
        text=True,
    )
    if result.returncode != 0:
        raise CommandError(invocation, result.returncode)
    return result.stdout


def tf(*args, debug=True, capture=False):
    return run("terraform", *args, cwd=TF_PROJECT, debug=debug, capture=capture)


def tf_vars():
    return ["-var", f"tg_bot_image_tag={project_version()}"]


@functools.lru_cache(maxsize=None)
def tf_output():
    return json.loads(tf("output", "--json", debug=False, capture=True))


@functools.lru_cache(maxsize=None)
def project_version():
    metadata = json.loads(
        run("cargo", "metadata", "--format-version", "1", "--no-deps", cwd=REPO, debug=False, capture=True)
    )
    return next(pkg["version"] for pkg in metadata["packages"] if pkg["name"] == "snowpity-tg")


def ssh_str():
    server = tf_output()["server"]["value"]
    return f"{server['os_user']}@{server['ip']}"


def ssh(*args, forward_ports=False):
    ports = []
    if forward_ports:
        ports = [
            "-L", "3000:localhost:3000",  # grafana
            "-L", "5000:localhost:5000",  # pgadmin
            "-L", "8428:localhost:8428",  # victoria-metrics
        ]  # fmt: skip
    run("ssh", "-t", ssh_str(), *ports, *args, debug=False)


def ssh_cloud_init_log(dump):
    cmd = "cat" if dump else "tail --follow"
    ssh(f"sudo {cmd} /var/log/cloud-init-output.log")


def docker_compose(*args, env=None):
    current_uid = {"CURRENT_UID": f"{os.getuid()}:{os.getgid()}"}
    run("docker", "compose", *args, cwd=REPO, env={**current_uid, **(env or {})})


def down(drop_data):
    docker_compose("down", "--timeout", "0", "--remove-orphans")
    if drop_data:
        try:
            run("docker", "volume", "rm", "snowpity_postgres")
        except CommandError:
            pass  # The volume does not exist.


def build_mode(release):
    return "release" if release else "debug"


def docker_build_all(release, push):
    mode = build_mode(release)
    info(f"Building in {mode} mode...")
    docker_build("tg-bot", push=push, context=".", build_args={"RUST_BUILD_MODE": mode})
    docker_build("grafana", push=push, context="./docker/grafana")


def docker_build(component, push, context, build_args=None):
    """Build an image with a tag of the exact version and the "latest" tag."""
    image = tf_output()["docker"]["value"]["image_name"]
    label = "" if component == "tg-bot" else f"{component}-"
    version_tag = f"{image}:{label}{project_version()}"
    latest_tag = f"{image}:{label}latest"

    pushing_msg = " and pushing it to the remote registry" if push else ""
    info(f"Building docker image {version_tag}{pushing_msg}")

    args = [
        "buildx", "build", context,
        "--file", f"docker/{component}/Dockerfile",
        "--tag", version_tag,
        "--tag", latest_tag,
        # We use ARM-propelled server in production, so doing AMD builds isn't critical
        "--platform", "linux/arm64/v8",
        "--push" if push else "--load",
    ]  # fmt: skip
    for name, value in (build_args or {}).items():
        args += ["--build-arg", f"{name}={value}"]

    run("docker", *args, cwd=REPO)


def with_retry(imp, delay=0.2, max_retries=5, max_delay=5.0, fixed=False):
    """Call `imp` until it succeeds or `max_retries` attempts fail.

    The delay grows exponentially with jitter, unless `fixed` is set.
    The last attempt raises its error.
    """
    base_delay = delay
    for attempt in range(1, max_retries):
        try:
            return imp()
        except CommandError as err:
            warn(err)
        warn(f"Failure! Retrying ({attempt} / {max_retries}) in {delay:.1f}s...")
        time.sleep(delay)
        if not fixed:
            delay = min(max_delay, random.uniform(base_delay, delay * 3))
    return imp()


def http_get_json(url):
    request = urllib.request.Request(url, headers={"User-Agent": "snowpity-x"})
    with urllib.request.urlopen(request) as response:
        return json.load(response)


def print_json(value):
    print(json.dumps(value, indent=2, ensure_ascii=False))


def log(color, level, msg):
    print(f"\033[{color};1m[{level}] {msg}\033[0m", file=sys.stderr)


def info(msg):
    log("32", "INFO", msg)


def warn(msg):
    log("33", "WARN", msg)


def error(msg):
    log("31", "ERROR", msg)


if __name__ == "__main__":
    main()
