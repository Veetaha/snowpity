# 0.4.0

## Changed

- Move the bot to Hetzner Cloud
- Scrape the server with grafana-agent sending telemetry to Grafana Cloud
- Add a script to SSH to Hetzner server
- Add a script to deploy the bot on Hetzner server and upgrade the docker image
- Define full infrastructure with terraform that include the deployment of the following:
    - Dockerhub repository
    - Grafana Cloud Stack
    - Hetzner server
- Add .editorconfig
