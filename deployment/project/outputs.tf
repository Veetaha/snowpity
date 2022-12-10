output "server" {
  value = {
    ip               = module.hetzner.server_ip
    status           = module.hetzner.server_status
    os_user          = module.hetzner.server_os_user
    data_volume_path = module.hetzner.data_volume_path
  }
}

output "grafana_cloud_stack" {
  value = {
    id                = grafana_cloud_stack.this.id
    status            = grafana_cloud_stack.this.status
    logs_status       = grafana_cloud_stack.this.logs_status
    prometheus_status = grafana_cloud_stack.this.prometheus_status
  }
}

output "grafana_loki_creds" {
  sensitive = true
  value = {
    loki_url       = grafana_cloud_stack.this.logs_url
    loki_ursername = grafana_cloud_stack.this.logs_user_id
    loki_password  = local.grafana_cloud_api_key
  }
}

output "docker" {
  value = {
    image_name = module.dockerhub.image_name
  }
}
