output "server_ip" {
  value = hcloud_server.master.ipv4_address
}

output "server_status" {
  value = hcloud_server.master.status
}

output "volume_mount_point" {
  value = local.volume_mount_point
}
