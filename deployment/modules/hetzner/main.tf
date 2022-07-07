locals {
  ssh_public_key = file("~/.ssh/id_rsa.pub")

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
  location    = "fsn1"
  ssh_keys    = [hcloud_ssh_key.admin.id]
  user_data   = data.cloudinit_config.master.rendered
}

resource "hcloud_ssh_key" "admin" {
  name       = "admin"
  public_key = local.ssh_public_key
}
