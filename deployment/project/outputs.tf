output "oci_compartment_id" {
  value = module.oci.compartment_id
}

output "server" {
  value = {
    id                      = module.oci.server_id
    ip                      = module.oci.server_ip
    state                   = module.oci.server_state
    data_volume_mount_point = module.oci.data_volume_mount_point
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
