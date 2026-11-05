locals {
  location = "hel1"

  hostname = "hetzner-master${module.workspace.id_suffix}"

  # XXX: using the name `admin` for the user is a bad idea. It does seem to work
  # fine on Hetzner. However, in the previous iterations of this project, when
  # we were using Oracle Cloud, it was found that `admin` user name causes the
  # server to be inaccessible via SSH. The supposition is that there is a conflict
  # with the `admin` group name already present in the used Oracle Ubuntu AMI.
  server_os_user = "mane"
}

module "workspace" {
  source = "../workspace"
}

resource "hcloud_server" "master" {
  name         = local.hostname
  image        = "ubuntu-24.04"
  server_type  = "cax21"
  location     = local.location
  user_data    = data.cloudinit_config.master.rendered
  firewall_ids = [hcloud_firewall.allow_only_ssh.id]

  public_net {
    # Not having IPv4 enabled reduces the cost, but we need it because we are
    # downloading some stuff from the public internet during the provisioning.
    ipv4_enabled = true
    ipv6_enabled = true
  }
}

resource "hcloud_volume" "master" {
  name     = "master-${local.location}${module.workspace.id_suffix}"
  size     = module.workspace.kind == "prod" ? 50 : 10
  location = local.location
}

resource "hcloud_volume_attachment" "master" {
  server_id = hcloud_server.master.id
  volume_id = hcloud_volume.master.id

  # Automount doesn't work if server's cloud-init script contains `runcmd` module
  # <https://github.com/hetznercloud/terraform-provider-hcloud/issues/473#issuecomment-971535629>
  # instead we use systemd mount unit via fstab
  automount = false
}

# The database must flush its data to the disk before the data volume is
# detached. Hetzner detaches the volume from a running server abruptly, and the
# `systemd` services that depend on the volume's mount are not stopped first.
# This resource depends on the volume attachment, so its destroy-time
# provisioner shuts the server down gracefully before the volume is detached.
resource "null_resource" "teardown" {
  triggers = {
    data_volume_attachment_id = hcloud_volume_attachment.master.id

    # A destroy-time provisioner can reference only `self`, so it gets its
    # inputs from the triggers.
    # Issue in terraform: https://github.com/hashicorp/terraform/issues/23679
    server_id    = hcloud_server.master.id
    hcloud_token = var.hcloud_token
  }

  lifecycle {
    # A token rotation must not replace this resource, because a replacement
    # shuts the server down.
    ignore_changes = [triggers["hcloud_token"]]
  }

  provisioner "local-exec" {
    when    = destroy
    command = "${path.module}/shutdown-server.py ${self.triggers.server_id}"

    environment = {
      HCLOUD_TOKEN = self.triggers.hcloud_token
    }
  }
}

resource "hcloud_firewall" "allow_only_ssh" {
  name = "allow-only-ssh"
  rule {
    direction  = "in"
    protocol   = "tcp"
    port       = "22"
    source_ips = var.allowed_ssh_ips
  }
}
