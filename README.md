[rust-toolchain]: https://www.rust-lang.org/tools/install

# veebot-telegram

This is a Telegram bot for me and friends.
It has assorted functionality for managing our Telegram chat.


# Development

To build the bot from sources, there has to be [Rust toolchain installed][rust-toolchain].

To build and run the bot in development mode run this:

```bash
cargo run
```

> ⚠️ Make sure to define all the necessary configurations in `.env` file when doing this. Example configurations can be inferred from [`deployment/modules/hetzner/main.tf`](deployment/modules/hetzner/main.tf).

It's also possible to run the bot in a container using `docker compose` just like it is going to be on the server. It requires some preliminary setup and passing of environment variables that are not expressible statically. Therefore, we have a dev CLI `cargo xtask`, that automates this process:

```bash
# Run the application in `docker`
cargo xtask start

# Stop any existing `docker-compose` stack
cargo xtask stop
```

# Deployment

The bot is deployed using [terraform]. All tf projects reside under `deployment/` directory.

The application is hosted by [Hetzner Cloud][hetzner]. It is delivered to the server via a [Dockerhub repository][dockerhub-repo], and covered with telemetry exfiltration by a [Grafana Cloud Stack][grafana-cloud]. The bot services are orchestrated by [docker-compose], which is in turn bootstrapped via [cloud-init] and [systemd].

To deploy the bot, you need to manually create several accounts at:

- [hub.docker.com](https://hub.docker.com/)
- [grafana.com](https://grafana.com/)
- [hetzner.com](https://www.hetzner.com/)

You also need to retrieve the bot token from [@BotFather] in Telegram.

Then, create a file `deployment/project/terraform.tfvars` with the secrets and credentials:

```hcl
tg_bot_token = {
    prod = "9999999999:AAaa9-9AAaa99AAaa99AAaa99AAaa99AAaa"
    dev  = "..."
}
hcloud_token = {
    prod = "AAaa99AAaa99AAaa99AAaa99AAaaAAaa99AAaa99AAaa99AAaa99AAaaAAaa99AA"
    dev  = "..."
}
grafana_cloud_api_key = {
    prod = "AAaa99AAaa99AAaa99AAaa99AAaaAAaa99AAaa99AAaa99AAaa99AAaaAAaa99AAAAaa99AAaa99AAa99AAAAaa99AAaa99AAa99AAAAaa9="
    dev  = "..."
}
docker_username = "username"
docker_password = "password"
```

Note that some credentials differ by the terraform workspace. If the default terraform workspace is selected, then `prod` credentials and configurations will be used. If `dev` workspace is selected, then development `dev` credentials and configurations will be used accordingly.

After that, you will be able to run the following command to deploy the entire stack with the Dockerhub repo, Hetzner server and Grafana Cloud Stack.

```bash
cd deployment/project && terraform apply
```

To clean everything up and get rid of the bot run:

```bash
cd deployment/project && terraform destroy
```

To destroy only part of the stack you can use [`--target` parameter][tf-targeting]. For example, to remove all Hetzner infrastructure run this:

> ⚠️ Warning! This will destroy the NAS volume of the database, basically resulting in data loss!

```bash
cd deployment/project terraform destroy --target module.hetzner
```

[terraform]: https://www.terraform.io/
[hetzner]: https://www.hetzner.com/
[dockerhub-repo]: https://hub.docker.com/repository/docker/veetaha/veebot-telegram
[grafana-cloud]: https://grafana.com/products/cloud/
[docker-compose]: https://docs.docker.com/compose/
[cloud-init]: https://cloudinit.readthedocs.io/en/latest/
[systemd]: https://www.freedesktop.org/wiki/Software/systemd/
[@BotFather]: https://core.telegram.org/bots
[tf-targeting]: https://www.terraform.io/cli/commands/plan#resource-targeting
