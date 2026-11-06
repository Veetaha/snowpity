# syntax = docker/dockerfile:1.2

FROM grafana/grafana:9.3.6

# Links the image to the repository in the GitHub Container Registry
LABEL org.opencontainers.image.source=https://github.com/Veetaha/snowpity

# Dashboards are very large, they don't fit into cloud-init, so we bake them
# into the image, and all other configs for consistency.
COPY provisioning /etc/grafana/provisioning
