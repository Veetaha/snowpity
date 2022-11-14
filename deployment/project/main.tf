locals {
  grafana_cloud_api_key = var.grafana_cloud_api_key[module.workspace.kind]
  tg_bot_token          = var.tg_bot_token[module.workspace.kind]
  hcloud_token          = var.hcloud_token[module.workspace.kind]
}

module "workspace" {
  source = "../modules/workspace"
}

module "hetzner" {
  source = "../modules/hetzner"

  tg_bot_token         = local.tg_bot_token
  veebot_tg_image_tag  = var.veebot_tg_image_tag
  veebot_tg_image_name = module.dockerhub.image_name

  prometheus_remote_write_url = grafana_cloud_stack.this.prometheus_remote_write_endpoint
  prometheus_username         = grafana_cloud_stack.this.prometheus_user_id
  prometheus_password         = local.grafana_cloud_api_key

  loki_url      = grafana_cloud_stack.this.logs_url
  loki_username = grafana_cloud_stack.this.logs_user_id
  loki_password = local.grafana_cloud_api_key

  pg_password      = var.pg_password
  pgadmin_password = var.pgadmin_password

  docker_username = var.docker_username
  docker_password = var.docker_password
}

module "grafana_cloud_stack" {
  source = "../modules/grafana_cloud_stack"
  providers = {
    grafana = grafana.cloud_stack
  }
}

module "dockerhub" {
  source = "../modules/dockerhub"
}
