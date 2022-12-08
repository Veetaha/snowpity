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

# Display the app's systemd service status
def "main ssh systemctl status" [] {
    ssh "sudo systemctl status tg-bot.service"
}

# Display the server's cloud-init logs
def "main ssh cloud-init log" [
    --dump # Don't show the tail of the log file, but dump its full contents
] {
    let log_file = '/var/log/cloud-init-output.log'
    let cmd = if $dump { "cat" } else { "tail --follow"}
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
    --code # Connect using VSCode
] {
    if not $code {
        ssh
        return
    }

    # FIXME: make vscode connection work via CLI, it doesn't right now :(
    code --folder-uri $"vscode-remote://ssh-remote+(ssh-str)"
}

def "main ssh str" [] {
    ssh-str
}

# Build the docker image with the app's executable
def "main docker build" [
    --release (-r) # Build in release mode
    --push (-p) # Push the image to remote docker registry
] {
    cd (repo)

    let image = (tf-output).docker.value.image_name

    let build_mode = if $release { "release" } else { "debug" }

    let version_tag = $"($image):(project-version)"
    let latest_tag = $"($image):latest"

    let pushing_msg = if $push { " and pushing it to the remote registry" } else { "" }

    info $"Building docker image ($version_tag) in ($build_mode) mode($pushing_msg)"

    let output_flag = if $push { "--push" } else { "--load" }

    (
        with-debug docker buildx build .
            '--tag' $version_tag
            '--tag' $latest_tag
            '--build-arg' $"RUST_BUILD_MODE=($build_mode)"
            # We use ARM-propelled server in oracle cloud, so doing AMD builds isn't critical
            '--platform' linux/arm64/v8
            $output_flag
    )
}

# Start all services locally using `docker compose`
def "main start" [
    --detach     # Run the services in the background
    --no-app     # Start only the database services. Useful when running app locally
    --fresh (-f) # Executes `db drop` before starting the database (run `db drop --help` for details)
] {
    if $fresh {
        info "--fresh was specified, so dropping the local database..."
        main db drop
    }

    mkdir (db-data)

    mut args = (
        [up --build]
        | append-if $detach '--wait'
        | append-if $no_app pg pgadmin
    )

    docker-compose $args
}

# Shutdown the local services using `docker compose`
def "main stop" [] {
    # We increase the timeout, because shutting down `teloxide` takes a while
    # The issue in `teloxide`: https://github.com/teloxide/teloxide/issues/711
    docker-compose down '--timeout' 60
}

# Clean the persistent data volume of the local database container
def "main db drop" [] {
    cd (repo)
    rm -rf (db-data)
    info $"Removed the database from (db-data)"
}

# Deploy the full application's stack
def "main deploy" [
    --no-build      # Skip build step, reuse the docker image that is already in the remote registry
    --release (-r)  # Build in release mode
    --drop-server   # Force the re-creation of the server instance
    --drop-db       # Drop the database (re-create the data volume)
    --plan          # Do `tf plan` instead of `tf apply`
    --yes (-y)      # Auto-approve the deployment
    --retry         # Retry the deployment if it fails until it succeeds
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
        | append-if $drop_server   '--replace=module.oci.oci_core_instance.master'
        | append-if $drop_db       '--replace=module.oci.oci_core_volume.master_data'
        | append-if $no_tf_refresh '--refresh=false'
    )

    if $retry {
        loop {
            try {
                tf $args
                break
            } catch { |err|
                let retry_duration = 2sec
                echo $err
                info $"Deployment failed, retrying in ($retry_duration)"
                sleep $retry_duration
            }
        }
    } else {
        tf $args
    }

    if not $plan {
        main ssh cloud-init log
    }
}

# Destroy the application's stack. By default destroys only the server instance,
# because it's safe to do, and no data will be lost. Use `--all` to destroy everything.
def "main destroy" [
    --yes (-y) # Auto-approve the destruction
    --all
    # Destroy all resources. ⚠️ This guarantees data loss because
    # the database's data volume will be destroyed as well
] {
    let args = (
        [destroy] ++ (tf-vars)
        | append-if $yes '--auto-approve'
        | append-if (not $all) '--target=module.oci.oci_core_instance.master'
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

# Generate the entities Rust code from the database schema. This will stop any
# local running containers, drop the database, re-create, migrate it and
# output the generated code to the working tree.
def "main orm gen" [] {
    cd (repo)
    main stop
    main start --fresh --no-app --detach
    wait-for-db
    sea-orm-cli migrate
    sea-orm-cli generate entity --with-copy-enums --output-dir entities/src/generated
    main stop
}

# Fetch the image metadata from derpibooru
def "main derpi image" [id:int] {
    fetch $"https://derpibooru.org/api/v1/json/images/($id)" | get image | flatten representations | get 0
}

################################################
############ Implementation details ############
################################################

# def-env database-url [] {
#     cached database-url {
#         open .env
#         | lines
#         | str trim
#         | where { |line| ($line | str length) != 0 }
#         | parse "{name}={value}"
#         | where name == DATABASE_URL
#         | get 0.value
#     }
# }

def-env tf-vars [] {
    ['-var' $'tg_bot_image_tag=(project-version)']
}

def-env repo [] {
    cached repo { git rev-parse --show-toplevel | str trim }
}

def-env db-data [] {
    cached db-data { $"(repo)/data/postgres" }
}

def-env tf-output [] {
    cached tf-output { tf --no-debug output '--json' | from json }
}

def-env cargo-metadata [] {
    cached cargo-metadata { cargo metadata --format-version 1 | from json }
}

def-env project-version [] {
    cargo-metadata | get packages | where name == snowpity-tg | get 0.version
}

def-env server-ip [] {
    (tf-output).server.value.ip
}

def-env ssh-str [] {
    let tf_output = tf-output
    let ip = $tf_output.server.value.ip
    let os_user = $tf_output.server.value.os_user

    $"($os_user)@($ip)"
}

def-env ssh [...args: string] {
    ^ssh -t (ssh-str) $args
}

def tf [--no-debug, ...args: string] {
    cd $"(repo)/deployment/project"

    let args = ($args | flatten-list)

    if $no_debug {
        return (terraform $args)
    }

    with-debug terraform $args
}

def docker-compose [--no-debug, ...args: any] {
    cd (repo)
    let current_uid = $"(id --user | str trim):(id --group | str trim)"
    let args = ($args | flatten-list | prepend compose)

    if $no_debug {
        return (CURRENT_UID=$current_uid docker $args)
    }

    CURRENT_UID=$current_uid with-debug docker $args
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

def with-debug [cmd: string, ...args: string] {
    let args = ($args | flatten-list)
    let invocation = $"($cmd) ($args | str join ' ')"

    debug $invocation

    let result = (run-external $cmd $args | complete)
    let span = (metadata $cmd).span;

    if $result.exit_code != 0 {
        let invocation = ([$invocation] | table --collapse)
        error make {
            msg: $"Command exited with code ($result.exit_code)\n\n($invocation)\n"
            label: {
                text: "The command originates from here"
                start: $span.start
                end: $span.end
            }
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
# This is `def-env` because it's the only way to mutate state in nushell.
def-env cached [cache_id: string, imp: block] {
    let cache_id = $'__cache_($cache_id)'

    let-env $cache_id = if $cache_id in $env {
        $env | get $cache_id
    } else {
        do $imp
    }

    $env | get $cache_id
}

def-env docker-compose-config [] {
    cached docker-compose-config {
        docker-compose --no-debug config '--format' json | from json
    }
}

def-env wait-for-db [] {
    let db_url = (
        docker-compose-config
        | get services.tg_bot.environment.DATABASE_URL
        | url parse
    )

    let db_name = ($db_url.path | parse "/{name}").0.name

    let postgres_image = (docker-compose-config).services.pg.image

    mut is_ready = false

    while not $is_ready {
        sleep 200ms
        $is_ready = (try {
            (
                with-debug docker run
                    '--network' 'snowpity_pg'
                    $postgres_image
                    pg_isready
                    '--dbname' $db_name
                    '--host' $db_url.host
                    '--port' $db_url.port
                    '--username' $db_url.username
            )
            true
        } catch { |e|
            false
        })
    }
}
