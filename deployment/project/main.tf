module "hetzner" {
  source = "../modules/hetzner"

  tg_bot_token        = var.tg_bot_token
  veebot_tg_image_tag = var.veebot_tg_image_tag

  prometheus_remote_write_url = grafana_cloud_stack.this.prometheus_remote_write_endpoint
  prometheus_username         = grafana_cloud_stack.this.prometheus_user_id
  prometheus_password         = var.grafana_cloud_api_key

  loki_url      = grafana_cloud_stack.this.logs_url
  loki_username = grafana_cloud_stack.this.logs_user_id
  loki_password = var.grafana_cloud_api_key

  pg_password      = var.pg_password
  pgadmin_password = var.pgadmin_password
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
