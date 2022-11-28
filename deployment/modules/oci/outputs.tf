output "compartment_id" {
  value = oci_identity_compartment.master.id
}

output "server_id" {
  value = oci_core_instance.master.id
}

output "server_ip" {
  value = oci_core_instance.master.public_ip
}

output "server_state" {
  value = oci_core_instance.master.state
}

output "data_volume_mount_point" {
  value = local.data_volume_mount_point
}

output "server_os_user" {
  value = local.server_os_user
}
