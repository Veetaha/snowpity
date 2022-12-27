[rust-toolchain]: https://www.rust-lang.org/tools/install

# Snowpity telegram bot

This is a Telegram bot for me and my friends.
It has assorted functionality for managing our Telegram chat.

If you wonder what the hell snowpity is, see [the explanation][snowpity-site].

> © MareQuest and [snowpity.com][snowpity-site] are't controlled by the maintainers of this telegram bot, any similarities are a coincidence!

# Development

To build the bot from sources, there has to be [Rust toolchain installed][rust-toolchain].

To build and run the bot in development mode outside of `docker` run this:

```bash
cargo run
```

Make sure to define all the necessary configurations in `.env` file when doing this. Example configurations can be inferred from [`deployment/modules/hetzner/cloud_init.tf`](deployment/modules/hetzner/cloud_init.tf).

It's also possible to run the bot in a container using `docker compose` just like it is going to be on the server. It requires some preliminary setup that is not expressible via a single `docker compose` command. Therefore, we have a dev CLI `x.nu`, that automates this process. We recommend adding an alias to your `.bashrc` or `.zshrc` or just `.{paste_your_shell_here}rc` file:

```bash
alias x="$(git rev-parse --show-toplevel)/x.nu"
```

You can use [`scripts/download/nu.sh`](scripts/download/nu.sh) to download the `nushell` interpreter for Linux OS. Then you'll need to place the `nu` binary somewhere in your `$PATH`.

After that you will be able to run:

```bash
# Run the application in `docker`, or only a subset of services
x up [--fresh|-f] [--no-tg-bot] [--no-observability]

# Teardown any existing `docker compose` stack
x down
```

# Deployment

The bot is deployed using [terraform]. All tf projects reside under `deployment/` directory.

The application is hosted on [Hetzner Cloud][hetzner-cloud]. It is delivered to the server via a [Dockerhub repository][dockerhub-repo], and covered with telemetry exfiltration by self-hosted [Grafana][grafana], [Grafana Agent][grafana-agent], [Loki][loki], [Victoria Metrics][victoria-metrics]. The bot services are orchestrated by [docker-compose], which is in turn bootstrapped via [cloud-init] and [systemd].

# Cost

Hetzner Cloud is not free, so you need some money 💰 on your credit card. You may tweak the server and volume sizes in terraform to optimize for your use case. With the configuration and market price at the time of this writing the entire stack in production costs **`€9.25/month`**, but if money is a big problem for you, then there are cheaper options.

The minimum Hetzner Cloud setup is **`€3.73/month`**, which can be achieved with the smallest possible server and volume size.

We use this minimum configuration for development mode deployments to save money 📈 there. This mode perfectly suits end-to-end testing of the app before pushing it to production.

## First time accounts setup

To deploy the bot, you need to manually create several accounts at:

- [hub.docker.com](https://hub.docker.com/)
- [hetzner.com/cloud][hetzner-cloud]

You also need to retrieve the bot token from [@BotFather] in Telegram.

Then, create a file `deployment/project/terraform.tfvars` with the secrets and credentials:

```hcl
tg_bot_maintainer = "999999999"
tg_bot_media_cache_chat = {
  prod = "-9999999999999"
  dev  = "..."
}
tg_bot_alerts_chat = {
  prod = "-9999999999999"
  dev = "..."
}
tg_bot_token = {
    prod = "9999999999:AAaa9-9AAaa99AAaa99AAaa99AAaa99AAaa"
    dev  = "..."
}

derpi_filter  = "100073"
derpi_api_key = "AAaa99AAaa99AAaa99AA"

docker_username = "username"
docker_password = "password"

pg_password      = "password"
pgadmin_password = "password"

hcloud_token = {
  prod = "AAaa99AAaa99AAaa99AAaa99AAaa99AAaa99AAaa99AAaa99AAaa99AAaa99AAaa"
  dev  = "..."
}

# IP addresses allowed to SSH in to the server by Hetzner firewall
allowed_ssh_ips = ["255.255.255.255/32"]
```

Note that some credentials differ by the terraform workspace. If the default terraform workspace is selected, then `prod` credentials and configurations will be used. If `dev` workspace is selected, then development `dev` credentials and configurations will be used accordingly.

## Deployment routine

After all the necessary credentials are configured, you will be able to run the following command to deploy the entire stack with the Dockerhub repo and Hetzner Cloud server.

Note, however, there currently is a chickend-and-egg problem, so the deployment won't succeed from the first try, because first the dockerhub repo must be deployed, then artifacts uploaded and only after that can we deploy the Hetzner server.

```bash
x deploy
```

To clean everything up and get rid of the bot entirely run the following command.

> ⚠️ Warning! This will destroy the NAS volume of the database, basically resulting in data loss!

```bash
x destroy --all
```

To destroy only the Hetzner Cloud server instance run this.

> ℹ This is safe to do. No data will be lost, the database will gracefully shutdown saving everything to the persistent NAS volume.
```bash
x destroy
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Snowpity by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>

[terraform]: https://www.terraform.io/
[hetzner-cloud]: https://www.hetzner.com/cloud

[dockerhub-repo]: https://hub.docker.com/repository/docker/veetaha/snowpity-tg
[grafana]: https://grafana.com
[grafana-agent]: https://grafana.com/docs/agent/latest/
[loki]: https://grafana.com/oss/loki/
[victoria-metrics]: https://victoriametrics.com/
[docker-compose]: https://docs.docker.com/compose/
[cloud-init]: https://cloudinit.readthedocs.io/en/latest/
[systemd]: https://www.freedesktop.org/wiki/Software/systemd/
[@BotFather]: https://core.telegram.org/bots
[snowpity-site]: https://snowpity.com/
