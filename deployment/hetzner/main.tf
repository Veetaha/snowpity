variable "hcloud_token" {
  sensitive = false
  type      = string
}

locals {
  ssh_public_key = file("~/.ssh/id_rsa.pub")
}

provider "hcloud" {
  token = var.hcloud_token
}

resource "hcloud_server" "master" {
  name        = "master"
  image       = "ubuntu-22.04"
  server_type = "cpx21"
  location    = "nbg1"
  ssh_keys    = [hcloud_ssh_key.admin.id]
  user_data   = templatefile("user_data.yml", { ssh_public_key = local.ssh_public_key })
}

resource "hcloud_ssh_key" "admin" {
  name       = "admin"
  public_key = local.ssh_public_key
}

output "server_ip" {
  value = hcloud_server.master.ipv4_address
}

output "server_status" {
  value = hcloud_server.master.status
}
