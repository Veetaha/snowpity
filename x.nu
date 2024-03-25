#!/usr/bin/env nu

# This is all a single file because it's not possible to use nushell modules from
# other files nicely. The module import paths are relative to the CWD
# of the script which is too much of a hassle to deal with.
# The issue in nushell about this is https://github.com/nushell/nushell/issues/7247
# FIXME: split this into files when the issue higher is fixed

# A dead-simple CLI that automates development processes in this repository,
# that are easily expressible via nushell scripts.
def main [] {}

#####################################
############ Entrypoints ############
#####################################

def "main ssh journalctl" [] {
    ssh "sudo journalctl --follow --catalog --unit tg-bot.service"
}

# Display the app's systemd service status
def "main ssh systemctl status" [] {
    ssh "sudo systemctl status tg-bot.service"
}

# Display the server's cloud-init logs
def "main ssh cloud-init log" [
    --dump # Don't show the tail of the log file, but dump its full contents
] {
    let log_file = '/var/log/cloud-init-output.log'
    let cmd = if $dump { "cat" } else { "tail --follow" }
    ssh $"sudo ($cmd) ($log_file)"
}

# Display the server's cloud-init status
def "main ssh cloud-init status" [] {
    ssh "cloud-init status"
}

# Show the ultimate user data residing on the server
def "main ssh cloud-init user-data" [] {
    ssh "sudo cloud-init query userdata"
}

# SSH into the app's server
def "main ssh" [
    --forward-ports (-f) # Forward grafana and pgadmin4 ports to localhost
    --code # Connect using VSCode
] {
    if not $code {
        if $forward_ports {
            ssh -f
        } else {
            ssh
        }
        return
    }

    # FIXME: make vscode connection work via CLI, it doesn't right now :(
    code --folder-uri $"vscode-remote://ssh-remote+(ssh-str)"
}

# Returns the user@ip string for the app's server
def "main ssh str" [] {
    ssh-str
}

# Build all docker images
def "main docker build" [
    --release (-r) # Build in release mode
    --push (-p) # Push the image to remote docker registry
] {
    cd (repo)

    let build_mode = if $release { "release" } else { "debug" }

    info $"Building in ($build_mode) mode..."

    docker-build tg-bot --push=$push --context . --build-args [[RUST_BUILD_MODE $build_mode]]
    docker-build grafana --push=$push --context ./docker/grafana
}

# Start all services locally using `docker compose`
def "main up" [
    --no-tg-bot        # Don't start the tg_bot service
    --no-observability # Don't start the pgadmin and observability services
    --fresh (-f)       # Executes `drop-data` before starting the database (run `db drop --help` for details)
    --release (-r)     # Build in release mode
] {
    cd (repo)

    let build_mode = if $release { "release" } else { "debug" }

    if $fresh {
        info "--fresh was specified, so deleting the data volumes..."
        main down --drop-data
    }

    let args = (
        [up --remove-orphans --build --wait postgres pgadmin]
        | append-if (not $no_tg_bot) tg-bot
        | append-if (not $no_observability) [
            victoria-metrics
            grafana
            loki
            grafana-agent
        ]
    )

    RUST_BUILD_MODE=$build_mode docker-compose $args

    if $no_tg_bot {
        with-debug sqlx migrate run '--source' crates/snowpity-tg/migrations
    }

    let args = (
        [logs '--follow']
        | append-if ($no_tg_bot) pgadmin postgres
        | append-if (not $no_tg_bot) '--no-log-prefix' tg-bot
    )

    docker-compose $args
}

# Shutdown the local containers and clean the persistent data volumes
def "main down" [
    --drop-data # Remove all data volumes
] {
    docker-compose down '--timeout' 0 '--remove-orphans'

    if $drop_data {
        try {
            with-debug docker volume rm snowpity_postgres
        } catch {
            # Ignore error if volume doesn't exist
        }
    }

}

# Deploy the full application's stack
def "main deploy" [
    --no-build      # Skip build step, reuse the docker image that is already in the remote registry
    --release (-r)  # Build in release mode
    --drop-server   # Force the re-creation of the server instance
    --drop-data     # Drop the persistent data (re-create the data volume)
    --plan          # Do `tf plan` instead of `tf apply`
    --yes (-y)      # Auto-approve the deployment
    --no-tf-refresh # Don't refresh the terraform state before deployment
] {
    if not $no_build {
        # FIXME: it's this verbose due to https://github.com/nushell/nushell/issues/7260
        if $release {
            main docker build --push --release
        } else {
            main docker build --push
        }
    }

    let args = if $plan { [plan] } else { [apply] }
    let args = (
        $args
        | append (tf-vars)
        | append-if $yes           '--auto-approve'
        | append-if $drop_server   '--replace=module.hetzner.hcloud_server.master'
        | append-if $drop_data     '--replace=module.hetzner.hcloud_volume.master'
        | append-if $no_tf_refresh '--refresh=false'
    )

    tf $args

    if not $plan {
        with-retry --max-retries 20 { main ssh cloud-init log }
    }
}

# Create a git tag with the current project's version and push it to the remote
def "main tag" [] {
    git tag $"v(project-version)"
    git push --tags
}

# Destroy the application's stack. By default destroys only the server instance,
# because it's safe to do, and no data will be lost. Use `--all` to destroy everything.
def "main destroy" [
    --yes (-y)  # Auto-approve the destruction
    --drop-data # Destroy the Hetzner Cloud volume. ⚠️ This guarantees data loss
    --all
    # Destroy all resources. ⚠️ This guarantees data loss because
    # the database's data volume will be destroyed as well
] {
    let args = (
        [destroy] ++ (tf-vars)
        | append-if $yes '--auto-approve'
        | append-if (not $all) '--target=module.hetzner.hcloud_server.master'
        | append-if (not $all and $drop_data) '--target=module.hetzner.hcloud_volume.master'
    )
    tf $args
}

# Convenience wrapper for `tf state list`
def "main tf state list" [] {
    tf --no-debug state list | lines
}

# Convenience wrapper for `tf output`
def "main tf output" [] {
    tf-output | to json | jq
}

# Fetch the image metadata from derpibooru
def "main derpibooru image" [id:int] {
    http get $"https://derpibooru.org/api/v1/json/images/($id)" | get image | flatten representations | get 0
}

# Get the ID of the telegram chat by its public tag.
# This is needed to get the ID of the private telegram chat or channel.
# We have to make that chat temporarily public, then get its ID and move
# it back to private again.
def "main tg chat-id" [
    --bot-token: string
    --chat-tag: string
] {
    http get $"https://api.telegram.org/bot($bot_token)/sendMessage?chat_id=@($chat_tag)&text=snowpity"
        | get result.chat.id
}

def "main sqlx prepare" [] {
    cd $"(repo)/crates/snowpity-tg"
    with-debug cargo sqlx prepare
}

# Invoke `cargo test` with logging enabled
def "main test" [...args: string] {
    cd (repo)
    let args = [test '--'] | append $args
    RUST_LOG="debug,h2=info,hyper=info" with-debug cargo $args
}

################################################
############ Implementation details ############
################################################

def --env tf-vars [] {
    ['-var' $'tg_bot_image_tag=(project-version)']
}

def --env repo [] {
    cached repo { git rev-parse --show-toplevel | str trim }
}

def --env tf-output [] {
    cached tf-output { tf --no-debug output '--json' | from json }
}

def --env cargo-metadata [] {
    # XXX: Caching cargo metadata causes a performance bug in nushell:
    # https://github.com/nushell/nushell/issues/6979#issuecomment-1343650021
    cargo metadata --format-version 1 | from json
}

def --env project-version [] {
    cargo-metadata | get packages | where name == snowpity-tg | get 0.version
}

def --env server-ip [] {
    (tf-output).server.value.ip
}

def --env ssh-str [] {
    let tf_output = tf-output
    let ip = $tf_output.server.value.ip
    let os_user = $tf_output.server.value.os_user

    $"($os_user)@($ip)"
}

def --env ssh [
    --forward-ports (-f) # Forward grafana and pgadmin4 ports to localhost
    ...args: string
] {
    let ports = [
        '-L' '3000:localhost:3000' # grafana
        '-L' '5000:localhost:5000' # pgadmin
        '-L' '8428:localhost:8428' # victria-metrics
    ]
    let args = (
        [-t (ssh-str)]
        | append-if $forward_ports $ports
        | append ($args | flatten-list)
    )

    ^ssh $args
}

def tf [--no-debug, ...args: string] {
    cd $"(repo)/deployment/project"

    let args = $args | flatten-list

    if $no_debug {
        return (terraform $args)
    }

    with-debug terraform $args
}

def docker-compose [--no-debug, ...args: any] {
    cd (repo)
    let current_uid = $"(id --user | str trim):(id --group | str trim)"
    let args = $args | flatten-list | prepend compose

    if $no_debug {
        return (CURRENT_UID=$current_uid docker $args)
    }

    CURRENT_UID=$current_uid with-debug docker $args
}

def warn [arg: any] {
    print --stderr $"(ansi yellow_bold)[WARN] ($arg) (ansi reset)"
}

def info [arg: any] {
    print --stderr $"(ansi green_bold)[INFO] ($arg) (ansi reset)"
}

def debug [arg: any] {
    print --stderr $"(ansi blue_bold)[DEBUG] ($arg) (ansi reset)"
}

def append-if [condition: bool, ...values: any] {
    if $condition { $in | append ($values | flatten-list) } else { $in }
}

def with-debug [cmd: string, ...args: any] {
    let args = $args | flatten-list
    let invocation = $"($cmd) ($args | str join ' ')"

    debug $invocation

    let result = (run-external --trim-end-newline $cmd $args | complete)
    let span = (metadata $cmd).span;

    if $result.exit_code != 0 {
        error make --unspanned {
            msg: $"Command exited with code ($result.exit_code)\n($invocation)"
        }
    }
}

def flatten-list [] {
    if (($in | length) == 1) and ($in.0 | describe | str starts-with "list<") {
        $in.0
    } else {
        $in
    }
}

# Some commands are expensive to run, and this utility may be used to cache
# their output.
#
# This is `def --env` because it's the only way to mutate state in nushell.
def --env cached [cache_id: string, imp: closure] {
    let cache_id = $'__cache_($cache_id)'

    load-env {
        $cache_id: (if $cache_id in $env {
            $env | get $cache_id
        } else {
            do $imp
        })
    }

    $env | get $cache_id
}

def --env docker-compose-config [] {
    cached docker-compose-config {
        docker-compose --no-debug config '--format' json | from json
    }
}

def --env wait-for-db [] {
    let db_url = (
        docker-compose-config
        | get services.tg-bot.environment.DATABASE_URL
        | url parse
    )

    let db_name = $db_url.path | parse "/{name}").0.name

    let postgres_image = (docker-compose-config).services.postgres.image

    let wait_time = 1min
    let delay = 200ms
    let max_retries = $wait_time / $delay | into int

    with-retry --fixed --max-retries $max_retries --delay $delay {(
        with-debug docker run
            '--network' 'snowpity_postgres'
            $postgres_image
            pg_isready
            '--dbname' $db_name
            '--host' $db_url.host
            '--port' $db_url.port
            '--username' $db_url.username
    )}
}

# Returns a pair of tags with the exact version and "latest" tag
def --env docker-build [
    component: string
    --push
    --build-args: list = []
    --context: string
] {
    cd (repo)

    let pushing_msg = if $push { " and pushing it to the remote registry" } else { "" }

    let output_flag = if $push { "--push" } else { "--load" }

    let image = (tf-output).docker.value.image_name

    let label = if $component == "tg-bot" { "" } else { $"($component)-" }

    let version_tag = $"($image):($label)(project-version)"
    let latest_tag = $"($image):($label)latest"

    info $"Building docker image ($version_tag)($pushing_msg)"

    let args = (
        [
            buildx build $context
            --file $"docker/($component)/Dockerfile"
            --tag $version_tag
            --tag $latest_tag
            # We use ARM-propelled server in production, so doing AMD builds isn't critical
            '--platform' linux/arm64/v8
            $output_flag
        ]
        | append ($build_args | each { |arg| ['--build-arg', $"($arg.0)=($arg.1)"] } | flatten)
    )

    with-debug docker $args
}

# Retry a closure until it succeeds or retry attempts are exhausted.
# Uses exponential backoff with jitter by default.
# See `--fixed` flag to alter this behavior.
def with-retry [
    imp: closure
    --delay       = 200ms # Initial delay between retries (will be fixed if --fixed is set)
    --max-retries = 5     # Maximum times to retry before giving up (first attempt is not counted)
    --max-delay   = 5sec  # Maximum delay between retries (only used with exponential backoff)
    --fixed               # Use fixed delay instead of exponential backoff
] {
    let exp = 3
    let base_delay = $delay
    mut attempt = 0
    mut delay = $delay

    loop {
        let success = (try {
            do $imp
            true
        } catch {|err|
            warn ($err | table)
            false
        })

        # XXX: nu currently doesn't support `return` in `try-catch`
        if $success {
            return
        }

        $attempt += 1

        warn $"Failure! Retrying \(($attempt) / ($max_retries)\) in ($delay)..."

        sleep $delay

        if $attempt >= $max_retries - 1 {
            # Do the last retry outside of the `try-catch`
            # to propagate the error
            break
        }

        if $fixed {
            continue
        }

        $delay = (
            [$max_delay, ((random int ($base_delay / 1ms)..($delay / 1ms * $exp)) * 1ms)]
            | math min
        )
    }

    do $imp
}
