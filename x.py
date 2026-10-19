#!/usr/bin/env python3
"""A dead-simple CLI that automates development processes in this repository."""

import argparse
import contextlib
import inspect
import json
import os
import random
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from collections.abc import Callable
from functools import cache
from inspect import Parameter
from pathlib import Path
from typing import Annotated, TypedDict, cast, get_args, get_origin

REPO = Path(__file__).resolve().parent
TF_PROJECT = REPO / "deployment" / "project"

type Json = dict[str, Json] | list[Json] | str | int | float | bool | None
type Command = Callable[..., None]

# The `x <name>` commands. Each function's docstring is its help, and its
# parameters are the arguments:
# - a `bool` is a flag;
# - a keyword-only parameter is an option;
# - another parameter is a positional argument;
# - `*args` takes the rest of the command line.
# `Annotated` adds a help string and short aliases such as "-r".
COMMANDS: dict[str, Command] = {}


def command[F: Command](name: str) -> Callable[[F], F]:
    def register(func: F) -> F:
        COMMANDS[name] = func
        return func

    return register


#####################################
############ Entrypoints ############
#####################################


@command("ssh")
def cmd_ssh(
    forward_ports: Annotated[bool, "-f", "Forward grafana and pgadmin4 ports to localhost"] = False,
    code: Annotated[bool, "Connect using VSCode"] = False,
) -> None:
    """SSH into the app's server"""
    if code:
        # FIXME: make vscode connection work via CLI, it doesn't right now :(
        run("code", "--folder-uri", f"vscode-remote://ssh-remote+{ssh_str()}")
    else:
        ssh(forward_ports=forward_ports)


@command("ssh str")
def cmd_ssh_str() -> None:
    """Print the user@ip string for the app's server"""
    print(ssh_str())


@command("ssh journalctl")
def cmd_ssh_journalctl() -> None:
    """Follow the logs of the app's systemd service"""
    ssh("sudo journalctl --follow --catalog --unit tg-bot.service")


@command("ssh systemctl status")
def cmd_ssh_systemctl_status() -> None:
    """Display the app's systemd service status"""
    ssh("sudo systemctl status tg-bot.service")


@command("ssh cloud-init log")
def cmd_ssh_cloud_init_log(
    dump: Annotated[bool, "Don't show the tail of the log file, but dump its full contents"] = False,
) -> None:
    """Display the server's cloud-init logs"""
    ssh_cloud_init_log(dump=dump)


@command("ssh cloud-init status")
def cmd_ssh_cloud_init_status() -> None:
    """Display the server's cloud-init status"""
    ssh("cloud-init status")


@command("ssh cloud-init user-data")
def cmd_ssh_cloud_init_user_data() -> None:
    """Show the ultimate user data residing on the server"""
    ssh("sudo cloud-init query userdata")


@command("docker build")
def cmd_docker_build(
    release: Annotated[bool, "-r", "Build in release mode"] = False,
    push: Annotated[bool, "-p", "Push the image to remote docker registry"] = False,
) -> None:
    """Build all docker images"""
    docker_build_all(release=release, push=push)


@command("up")
def cmd_up(
    no_tg_bot: Annotated[bool, "Don't start the tg_bot service"] = False,
    no_observability: Annotated[bool, "Don't start the pgadmin and observability services"] = False,
    fresh: Annotated[bool, "-f", "Delete the data volumes before starting the database"] = False,
    release: Annotated[bool, "-r", "Build in release mode"] = False,
) -> None:
    """Start all services locally using `docker compose`"""
    if fresh:
        info("--fresh was specified, so deleting the data volumes...")
        down(drop_data=True)

    services = ["postgres", "pgadmin"]
    if not no_tg_bot:
        services.append("tg-bot")
    if not no_observability:
        services += ["victoria-metrics", "grafana", "loki", "grafana-agent"]

    docker_compose(
        "up", "--remove-orphans", "--build", "--wait", *services,
        env={"RUST_BUILD_MODE": build_mode(release)},
    )  # fmt: skip

    if no_tg_bot:
        run("sqlx", "migrate", "run", "--source", "crates/snowpity-tg/migrations", cwd=REPO)
        docker_compose("logs", "--follow", "pgadmin", "postgres")
    else:
        docker_compose("logs", "--follow", "--no-log-prefix", "tg-bot")


@command("down")
def cmd_down(drop_data: Annotated[bool, "Remove all data volumes"] = False) -> None:
    """Shutdown the local containers and clean the persistent data volumes"""
    down(drop_data=drop_data)


@command("deploy")
def cmd_deploy(
    no_build: Annotated[bool, "Skip build step, reuse the docker image that is already in the remote registry"] = False,
    release: Annotated[bool, "-r", "Build in release mode"] = False,
    drop_server: Annotated[bool, "Force the re-creation of the server instance"] = False,
    drop_data: Annotated[bool, "Drop the persistent data (re-create the data volume)"] = False,
    plan: Annotated[bool, "Do `tf plan` instead of `tf apply`"] = False,
    yes: Annotated[bool, "-y", "Auto-approve the deployment"] = False,
    no_tf_refresh: Annotated[bool, "Don't refresh the terraform state before deployment"] = False,
) -> None:
    """Deploy the full application's stack"""
    if not no_build:
        docker_build_all(release=release, push=True)

    args = ["plan" if plan else "apply", *tf_vars()]
    if yes:
        args.append("--auto-approve")
    if drop_server:
        args.append("--replace=module.hetzner.hcloud_server.master")
    if drop_data:
        args.append("--replace=module.hetzner.hcloud_volume.master")
    if no_tf_refresh:
        args.append("--refresh=false")

    tf(*args)

    if not plan:
        # The server boots for a while, and ssh fails until it is up.
        with_retry(lambda: ssh_cloud_init_log(dump=False), max_retries=20)


@command("tag")
def cmd_tag() -> None:
    """Create a git tag with the current project's version and push it to the remote"""
    run("git", "tag", f"v{project_version()}", cwd=REPO)
    run("git", "push", "--tags", cwd=REPO)


@command("destroy")
def cmd_destroy(
    yes: Annotated[bool, "-y", "Auto-approve the destruction"] = False,
    drop_data: Annotated[bool, "Destroy the Hetzner Cloud volume. ⚠️ This guarantees data loss"] = False,
    all: Annotated[
        bool,
        "Destroy all resources. ⚠️ This guarantees data loss because "
        "the database's data volume will be destroyed as well",
    ] = False,
) -> None:
    """Destroy the application's stack. By default destroys only the server instance,
    because it's safe to do, and no data will be lost. Use `--all` to destroy everything."""
    args = ["destroy", *tf_vars()]
    if yes:
        args.append("--auto-approve")
    if not all:
        args.append("--target=module.hetzner.hcloud_server.master")
        if drop_data:
            args.append("--target=module.hetzner.hcloud_volume.master")
    tf(*args)


@command("tf state list")
def cmd_tf_state_list() -> None:
    """Convenience wrapper for `tf state list`"""
    tf("state", "list", debug=False)


@command("tf output")
def cmd_tf_output() -> None:
    """Convenience wrapper for `tf output`"""
    print_json(tf_output())


class DerpibooruImageResponse(TypedDict):
    image: dict[str, Json]


@command("derpibooru image")
def cmd_derpibooru_image(id: int) -> None:
    """Fetch the image metadata from derpibooru"""
    url = f"https://derpibooru.org/api/v1/json/images/{id}"
    image = http_get_json(url, DerpibooruImageResponse)["image"]
    # Show the image URLs of each size next to the other fields.
    representations = image.pop("representations")
    assert isinstance(representations, dict)
    print_json(image | representations)


class TelegramChat(TypedDict):
    id: int


class TelegramMessage(TypedDict):
    chat: TelegramChat


class TelegramResponse(TypedDict):
    result: TelegramMessage


@command("tg chat-id")
def cmd_tg_chat_id(*, bot_token: str, chat_tag: str) -> None:
    """Get the ID of the telegram chat by its public tag.
    This is needed to get the ID of the private telegram chat or channel.
    We have to make that chat temporarily public, then get its ID and move
    it back to private again."""
    query = urllib.parse.urlencode({"chat_id": f"@{chat_tag}", "text": "snowpity"})
    url = f"https://api.telegram.org/bot{bot_token}/sendMessage?{query}"
    print(http_get_json(url, TelegramResponse)["result"]["chat"]["id"])


@command("sqlx prepare")
def cmd_sqlx_prepare() -> None:
    """Run `cargo sqlx prepare` for the bot crate"""
    run("cargo", "sqlx", "prepare", cwd=REPO / "crates" / "snowpity-tg")


@command("test")
def cmd_test(*args: Annotated[str, "Arguments for the test binary"]) -> None:
    """Invoke `cargo test` with logging enabled"""
    run("cargo", "test", "--", *args, cwd=REPO, env={"RUST_LOG": "debug,h2=info,hyper=info"})


def main() -> None:
    parser = cli()
    # argparse does not start `*args` with an option, such as `x test --nocapture`.
    # It leaves such options in `unknown`, before the rest of `*args`.
    args, unknown = parser.parse_known_args()
    func = cast(Command, args.func)
    params = inspect.signature(func).parameters.values()
    if unknown and all(param.kind is not Parameter.VAR_POSITIONAL for param in params):
        parser.error(f"unrecognized arguments: {' '.join(unknown)}")

    positional: list[object] = []
    keyword: dict[str, object] = {}
    for param in params:
        value = cast(object, getattr(args, param.name))
        match param.kind:
            case Parameter.VAR_POSITIONAL:
                rest = unknown + cast(list[str], value)
                positional += rest[1:] if rest[:1] == ["--"] else rest
            case Parameter.KEYWORD_ONLY:
                keyword[param.name] = value
            case _:
                positional.append(value)
    try:
        func(*positional, **keyword)
    except CommandError as err:
        error(err)
        sys.exit(err.exit_code)
    except KeyboardInterrupt:
        sys.exit(130)


################################################
############ Implementation details ############
################################################


def cli() -> argparse.ArgumentParser:
    """Build the parser of `COMMANDS`."""
    # Each command path with the paths of its groups, such as "ssh cloud-init".
    paths = dict.fromkeys(
        " ".join(words[:end]) for words in (path.split() for path in COMMANDS) for end in range(1, len(words) + 1)
    )

    def add_commands(parser: argparse.ArgumentParser, parent: str) -> None:
        children = [path for path in paths if path.rpartition(" ")[0] == parent]
        if not children:
            return
        subcommands = parser.add_subparsers(metavar="<command>", required=parent not in COMMANDS)
        for path in children:
            func = COMMANDS.get(path)
            help = inspect.getdoc(func) if func else None
            child = subcommands.add_parser(
                path.rpartition(" ")[2], help=help, description=help, allow_abbrev=False, suggest_on_error=True
            )
            if func:
                child.set_defaults(func=func)
                for param in inspect.signature(func).parameters.values():
                    add_argument(child, param)
            add_commands(child, path)

    root = argparse.ArgumentParser(prog="x", description=__doc__, allow_abbrev=False, suggest_on_error=True)
    add_commands(root, "")
    return root


def add_argument(parser: argparse.ArgumentParser, param: Parameter) -> None:
    annotation = cast(object, param.annotation)
    if get_origin(annotation) is Annotated:
        type_, *meta = cast(tuple[object, ...], get_args(annotation))
    else:
        type_, meta = annotation, []
    aliases = [m for m in meta if isinstance(m, str) and m.startswith("-")]
    help = next((m for m in meta if isinstance(m, str) and not m.startswith("-")), None)
    option = "--" + param.name.replace("_", "-")
    parse = cast(Callable[[str], object], type_)

    match param.kind:
        case Parameter.VAR_POSITIONAL:
            parser.add_argument(param.name, nargs=argparse.REMAINDER, help=help)
        case _ if type_ is bool:
            parser.add_argument(*aliases, option, dest=param.name, action="store_true", help=help)
        case Parameter.KEYWORD_ONLY:
            required = param.default is Parameter.empty
            default = None if required else cast(object, param.default)
            parser.add_argument(
                *aliases, option, dest=param.name, type=parse, required=required, default=default, help=help
            )
        case _:
            parser.add_argument(param.name, type=parse, help=help)


class CommandError(Exception):
    def __init__(self, invocation: str, exit_code: int) -> None:
        super().__init__(f"Command exited with code {exit_code}\n{invocation}")
        self.exit_code = exit_code


def run(*cmd: str | Path, cwd: Path | None = None, env: dict[str, str] | None = None, debug: bool = True) -> None:
    """Run a command and raise `CommandError` when it fails."""
    argv = [str(arg) for arg in cmd]
    invocation = " ".join(argv)
    if debug:
        log("34", "DEBUG", invocation)
    result = subprocess.run(argv, check=False, cwd=cwd, env=os.environ | env if env else None)
    if result.returncode != 0:
        raise CommandError(invocation, result.returncode)


def output(*cmd: str, cwd: Path) -> str:
    """Run a command and return its stdout. Raise `CommandError` when it fails."""
    result = subprocess.run(cmd, check=False, cwd=cwd, stdout=subprocess.PIPE, text=True)
    if result.returncode != 0:
        raise CommandError(" ".join(cmd), result.returncode)
    return result.stdout


def tf(*args: str, debug: bool = True) -> None:
    run("terraform", *args, cwd=TF_PROJECT, debug=debug)


def tf_vars() -> list[str]:
    return ["-var", f"tg_bot_image_tag={project_version()}"]


class TfValue[T](TypedDict):
    value: T


class TfServer(TypedDict):
    ip: str
    os_user: str


class TfDocker(TypedDict):
    image_name: str


class TfOutput(TypedDict):
    server: TfValue[TfServer]
    docker: TfValue[TfDocker]


@cache
def tf_output() -> TfOutput:
    return parse_json(output("terraform", "output", "--json", cwd=TF_PROJECT), TfOutput)


class CargoPackage(TypedDict):
    name: str
    version: str


class CargoMetadata(TypedDict):
    packages: list[CargoPackage]


@cache
def project_version() -> str:
    metadata = output("cargo", "metadata", "--format-version", "1", "--no-deps", cwd=REPO)
    packages = parse_json(metadata, CargoMetadata)["packages"]
    return next(pkg["version"] for pkg in packages if pkg["name"] == "snowpity-tg")


def ssh_str() -> str:
    server = tf_output()["server"]["value"]
    return f"{server['os_user']}@{server['ip']}"


def ssh(*args: str, forward_ports: bool = False) -> None:
    ports: list[str] = []
    if forward_ports:
        ports = [
            "-L", "3000:localhost:3000",  # grafana
            "-L", "5000:localhost:5000",  # pgadmin
            "-L", "8428:localhost:8428",  # victoria-metrics
        ]  # fmt: skip
    run("ssh", "-t", ssh_str(), *ports, *args, debug=False)


def ssh_cloud_init_log(dump: bool) -> None:
    cmd = "cat" if dump else "tail --follow"
    ssh(f"sudo {cmd} /var/log/cloud-init-output.log")


def docker_compose(*args: str, env: dict[str, str] | None = None) -> None:
    current_uid = {"CURRENT_UID": f"{os.getuid()}:{os.getgid()}"}
    run("docker", "compose", *args, cwd=REPO, env=current_uid | (env or {}))


def down(drop_data: bool) -> None:
    docker_compose("down", "--timeout", "0", "--remove-orphans")
    if drop_data:
        # The volume may not exist.
        with contextlib.suppress(CommandError):
            run("docker", "volume", "rm", "snowpity_postgres")


def build_mode(release: bool) -> str:
    return "release" if release else "debug"


def docker_build_all(release: bool, push: bool) -> None:
    mode = build_mode(release)
    info(f"Building in {mode} mode...")
    docker_build("tg-bot", push=push, context=".", build_args={"RUST_BUILD_MODE": mode})
    docker_build("grafana", push=push, context="./docker/grafana")


def docker_build(component: str, push: bool, context: str, build_args: dict[str, str] | None = None) -> None:
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


def with_retry[T](
    imp: Callable[[], T],
    *,
    delay: float = 0.2,
    max_retries: int = 5,
    max_delay: float = 5.0,
    fixed: bool = False,
) -> T:
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


def parse_json[T](text: str, schema: type[T]) -> T:
    """Parse JSON that has the shape of `schema`. The shape is not checked."""
    return cast(T, json.loads(text))


def http_get_json[T](url: str, schema: type[T]) -> T:
    request = urllib.request.Request(url, headers={"User-Agent": "snowpity-x"})
    with urllib.request.urlopen(request) as response:
        return parse_json(response.read().decode(), schema)


def print_json(value: object) -> None:
    print(json.dumps(value, indent=2, ensure_ascii=False))


def log(color: str, level: str, msg: object) -> None:
    print(f"\033[{color};1m[{level}] {msg}\033[0m", file=sys.stderr)


def info(msg: object) -> None:
    log("32", "INFO", msg)


def warn(msg: object) -> None:
    log("33", "WARN", msg)


def error(msg: object) -> None:
    log("31", "ERROR", msg)


if __name__ == "__main__":
    main()
