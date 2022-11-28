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
    ssh "sudo systemctl status veebot-tg.service"
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
    if $code {
        # FIXME: make vscode connection work via CLI, it doesn't right now :(
        # code --folder-uri $"vscode-remote://ssh-remote+(ssh-str)"
    } else {
        ssh
    }
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

# Start the local database container with pgadmin using `docker compose`
def "main db start" [
    --fresh (-f) # Executes `db drop` before starting the database (run `db drop --help` for details)
] {
    if $fresh {
        info "--fresh was speicified, so dropping the local database..."
        main db drop
    }

    mkdir (db-data)
    docker-compose up postgres pgadmin
}

# Shutdown the local database container
def "main db stop" [] {
    docker-compose down
}

# Clean the persistent data volume of the local database container
def "main db drop" [] {
    cd (repo)
    rm -rf (db-data)
    info $"Removed the database from (db-data)"
}

# Deploy the full application's stack
def "main deploy" [
    --no-build     # Skip build step, reuse the docker image that is already in the remote registry
    --release (-r) # Build in release mode
    --drop-server  # Force the re-creation of the server instance
    --drop-db      # Drop the database (re-create the data volume)
] {
    if not $no_build {
        # FIXME: it's this verbose due to https://github.com/nushell/nushell/issues/7260
        if $release {
            main docker build --push --release
        } else {
            main docker build --push
        }
    }

    let args = ['apply' '-auto-approve' '-var' $'veebot_tg_image_tag=(project-version)']
    let args = ($args | append-if $drop_server '--replace=module.oci.oci_core_instance.master')
    let args = ($args | append-if $drop_db     '--replace=module.oci.oci_core_volume.master_data')

    tf $args

    main ssh cloud-init log
}

# Convenience wrapper for `tf state list`
def "main tf state list" [] {
    tf state list | lines
}

################################################
############ Implementation details ############
################################################

# This is an env variable, because it's the only way to mutate state in nushell.
# We need this to lazily cache the server IP on the first usage.

def-env cached [cache_id: string, imp: block] {
    let cache_id = $'__cache_($cache_id)'

    let-env $cache_id = if $cache_id in $env {
        $env | get $cache_id
    } else {
        do $imp
    }

    $env | get $cache_id
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
    cargo-metadata | get packages | where name == veebot-telegram | get 0.version
}

def-env ssh-str [] {
    let tf_output = tf-output
    let ip = $tf_output.server.value.ip
    let os_user = $tf_output.server.value.os_user

    $"($os_user)@($ip)"
}

def-env ssh [...args: string] {
    ^ssh (ssh-str) $args
}

def tf [--no-debug, ...args: string] {
    cd $"(repo)/deployment/project"

    let args = ($args | flatten-list)

    if $no_debug {
        terraform $args
    } else {
        with-debug terraform $args
    }
}

def docker-compose [...args: string] {
    cd (repo)
    (
        CURRENT_UID=$"(id --user | str trim):(id --group | str trim)"
        with-debug docker ($args | flatten-list | prepend compose)
    )
}

def info [arg: any] {
    print --stderr $"(ansi green_bold)[INFO] ($arg) (ansi reset)"
}

def debug [arg: any] {
    print --stderr $"(ansi blue_bold)[DEBUG] ($arg) (ansi reset)"
}

def append-if [condition: bool, value: any] {
    if $condition { $in | append [$value] } else { $in }
}

def with-debug [cmd: string, ...args: string] {
    let args = ($args | flatten-list)
    debug $"($cmd) ($args | str join ' ')"
    run-external --redirect-stdout $cmd $args
}

def flatten-list [] {
    if (($in | length) == 1) and ($in.0 | describe | str starts-with "list<") {
        $in.0
    } else {
        $in
    }
}
