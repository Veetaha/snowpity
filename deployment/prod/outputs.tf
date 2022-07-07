output "server" {
  value = {
    ip     = module.hetzner.server_ip
    status = module.hetzner.server_status
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
