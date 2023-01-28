# syntax = docker/dockerfile:1.2

FROM grafana/grafana:9.3.6

# Dashboards are very large, they don't fit into cloud-init, so we bake them
# into the image, and all other configs for consistency.
COPY provisioning /etc/grafana/provisioning
