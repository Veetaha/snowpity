# 0.4.1

## Changed

- Updated captcha question to a more straighforward one without swearwords

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
