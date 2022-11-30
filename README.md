[rust-toolchain]: https://www.rust-lang.org/tools/install

# Snowpity telegram bot

This is a Telegram bot for me and friends.
It has assorted functionality for managing our Telegram chat.

# Development

To build the bot from sources, there has to be [Rust toolchain installed][rust-toolchain].

To build and run the bot in development mode run this:

```bash
cargo run
```

> ⚠️ Make sure to define all the necessary configurations in `.env` file when doing this. Example configurations can be inferred from [`deployment/modules/oci/main.tf`](deployment/modules/oci/main.tf).

It's also possible to run the bot in a container using `docker compose` just like it is going to be on the server. It requires some preliminary setup and passing of environment variables that are not expressible statically. Therefore, we have a dev CLI `x.nu`, that automates this process. We recommend adding an alias to your `.bashrc` or `.zshrc` file:

```bash
alias x="$(git rev-parse --show-toplevel)/x.nu"
```

You can use [`scripts/download/nu.sh`](scripts/download/nu.sh) to download the `nushell` interpreter on Linux OS. Then you'll need to place the `nu` binary somewhere in your `$PATH`.

After that you will be able to run:

```bash
# Run the application in `docker`, or only the database if `--no-app` was specified
x start [--detach] [--no-app]

# Stop any existing `docker-compose` stack
x stop
```

# Deployment

The bot is deployed using [terraform]. All tf projects reside under `deployment/` directory.

The application is hosted on [Oracle Cloud][oracle-cloud] using [Always Free Tier][oci-always-free]. It is delivered to the server via a [Dockerhub repository][dockerhub-repo], and covered with telemetry exfiltration by a [Grafana Cloud Stack][grafana-cloud]. The bot services are orchestrated by [docker-compose], which is in turn bootstrapped via [cloud-init] and [systemd].

## First time accounts setup

To deploy the bot, you need to manually create several accounts at:

- [hub.docker.com](https://hub.docker.com/)
- [grafana.com](https://grafana.com/)
- [oracle.com/cloud][oracle-cloud]

You also need to retrieve the bot token from [@BotFather] in Telegram.

Then, create a file `deployment/project/terraform.tfvars` with the secrets and credentials:

```hcl
tg_bot_token = {
    prod = "9999999999:AAaa9-9AAaa99AAaa99AAaa99AAaa99AAaa"
    dev  = "..."
}

oci_parent_compartment_id = "ocid1.tenancy.oc1..aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

grafana_cloud_api_key = {
    prod = "AAaa99AAaa99AAaa99AAaa99AAaaAAaa99AAaa99AAaa99AAaa99AAaaAAaa99AAAAaa99AAaa99AAa99AAAAaa99AAaa99AAa99AAAAaa9="
    dev  = "..."
}
docker_username = "username"
docker_password = "password"
```

Note that some credentials differ by the terraform workspace. If the default terraform workspace is selected, then `prod` credentials and configurations will be used. If `dev` workspace is selected, then development `dev` credentials and configurations will be used accordingly.

Additionally, you need to create the following config file in `~/.oci/config` with Oracle Cloud creds.

```ini
[DEFAULT]
fingerprint=aa:aa:aa:aa:aa:aa:aa:aa:aa:aa:aa:aa:aa:aa:aa:aa
key_file=~/.oci/oci_api_key.pem
region=eu-frankfurt-1
tenancy=ocid1.tenancy.oc1..aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
user=ocid1.user.oc1..aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
```

The `oci_api_key.pem` file can be generated from Oracle Cloud web UI ([docs link](https://docs.oracle.com/en-us/iaas/Content/API/Concepts/apisigningkey.htm)).

## Deployment routine

After all the necessary credentials are configured, you will be able to run the following command to deploy the entire stack with the Dockerhub repo, Hetzner server and Grafana Cloud Stack.

```bash
x deploy
```

To clean everything up and get rid of the bot entirely run the following command.

> ⚠️ Warning! This will destroy the NAS volume of the database, basically resulting in data loss!

```bash
x destroy
```

To destroy only the Oracle Cloud server instance run this.

> ℹ This is safe to do. No data will be lost, the database will gracefully shutdown saving everything to the persistent NAS volume.
```bash
x destroy server
```

[terraform]: https://www.terraform.io/
[oracle-cloud]: https://www.oracle.com/cloud/
[oci-always-free]: https://www.oracle.com/cloud/free/
[dockerhub-repo]: https://hub.docker.com/repository/docker/veetaha/snowpity-tg
[grafana-cloud]: https://grafana.com/products/cloud/
[docker-compose]: https://docs.docker.com/compose/
[cloud-init]: https://cloudinit.readthedocs.io/en/latest/
[systemd]: https://www.freedesktop.org/wiki/Software/systemd/
[@BotFather]: https://core.telegram.org/bots
