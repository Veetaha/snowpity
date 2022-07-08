locals {
  volume_mount_point = "/mnt/master"
  volume_fs          = "ext4"
  location           = "fsn1"
  ssh_public_key     = file("~/.ssh/id_rsa.pub")

  grafana_agent_vars = {
    prometheus_remote_write_url = var.prometheus_remote_write_url
    prometheus_username         = var.prometheus_username
    prometheus_password         = var.prometheus_password

    loki_remote_write_url = var.loki_remote_write_url
    loki_username         = var.loki_username
    loki_password         = var.loki_password
  }

  grafana_agent_config = templatefile("${path.module}/grafana-agent.yaml", local.grafana_agent_vars)

  user_data_vars = {
    ssh_public_key     = local.ssh_public_key
    grafana_agent_yaml = base64gzip(local.grafana_agent_config)
    volume_device      = hcloud_volume.master.linux_device
    volume_mount_point = local.volume_mount_point
    volume_fs          = local.volume_fs
  }
}

data "cloudinit_config" "master" {
  part {
    content = templatefile("${path.module}/user_data.yaml", local.user_data_vars)
  }
}

resource "hcloud_server" "master" {
  name        = "master"
  image       = "ubuntu-22.04"
  server_type = "cpx21"
  location    = local.location
  ssh_keys    = [hcloud_ssh_key.admin.id]
  user_data   = data.cloudinit_config.master.rendered
}

resource "hcloud_ssh_key" "admin" {
  name       = "admin"
  public_key = local.ssh_public_key
}

resource "hcloud_volume" "master" {
  name     = "master"
  size     = 50
  location = local.location
}

resource "hcloud_volume_attachment" "master" {
  server_id = hcloud_server.master.id
  volume_id = hcloud_volume.master.id

  # automount doesn't work if server's cloud-init script contains `runcmd` module
  # <https://github.com/hetznercloud/terraform-provider-hcloud/issues/473#issuecomment-971535629>
  automount = false
}
