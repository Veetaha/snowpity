output "server" {
  value = {
    ip               = module.hetzner.server_ip
    status           = module.hetzner.server_status
    os_user          = module.hetzner.server_os_user
    data_volume_path = module.hetzner.data_volume_path
  }
}

output "docker" {
  value = {
    image_name = module.dockerhub.image_name
  }
}
