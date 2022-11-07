# 0.4.2

## Changed

- Updated `teloxide` from `0.10` to `0.11`. This should fix some parsing bugs
- Improved logging in captcha module
- Made the teardown in development mode instant by skipping teloxide's shutdown logic
- Waiting for tracing-loki to flush logs for 3 seconds before teardown heuristically in release mode
- Changed the deployment folder to use terraform workspace for production and development
- Started working on the ingretaion of the database and censoring logic (not finished yet)
- Migrated to Rust 1.65.0
- Added maintainer `/details` command for debugging of the messages and their senders

# 0.4.1

## Changed

- Updated captcha question to a more straightforward one without swearwords

# 0.4.0

## Changed

- Move the bot to Hetzner Cloud
- Scrape the server with grafana-agent sending telemetry to Grafana Cloud
    - The Linux server is configured with node_exporter only
- Add a script to SSH to Hetzner server
- Add a script to deploy the bot on Hetzner server and upgrade the docker image
- Define full infrastructure with terraform that include the deployment of the following:
    - Dockerhub repository
    - Grafana Cloud Stack
    - Hetzner server
- Add .editorconfig
- Add `cargo xtask` utility for development in docker-compose environment
- Add `tracing_loki` layer to the applicating to send logs to Grafana Cloud
- Add a NAS volume for Hetzner server to use as persistent storage for future Postgres instance
