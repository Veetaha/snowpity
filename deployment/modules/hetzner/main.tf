locals {
  log_filter = [
    "debug",
    "hyper=info",
    "reqwest=info",
    "rustls=info",
    "sqlx=warn",
    "h2=info",
    # Don't log HTTP requests to `/metrics` endpoint, which we know
    # will be hit by Prometheus regularly, but log all other ones,
    # because we don't expect any other incomming HTTP traffic.
    "snowpity::tg_metrics[uri=\"/metrics\"]=off",
    "snowpity::tg_metrics=trace",
  ]

  location         = "fsn1"
  hostname         = "hetzner-master${module.workspace.id_suffix}"
  data_volume_path = "/mnt/master"
  data_volume_fs   = "ext4"
  pg_data          = "${local.data_volume_path}/data/postgres"
  env_file_path    = "/var/app/.env"
  systemd_service  = "tg-bot.service"

  # XXX: using the name `admin` for the user is a bad idea. It does seem to work
  # fine on Hetzner. However, in the previous iterations of this project, when
  # we were using Oracle Cloud, it was found that `admin` user name causes the
  # server to be inaccessible via SSH. The supposition is that there is a conflict
  # with the `admin` group name already present in the used Oracle Ubuntu AMI.
  server_os_user = "mane"

  templates = {
    "grafana-agent.yaml"    = "/etc/grafana-agent.yaml"
    (local.systemd_service) = "/etc/systemd/system/tg-bot.service"
    "data-volume.service"   = "/etc/systemd/system/data-volume.service"
  }

  exec_files = {
    "/var/app/docker-compose.sh" = file("${path.module}/templates/docker-compose.sh")
    "/var/app/data-volume.sh"    = file("${path.module}/templates/data-volume.sh")
  }

  data_files = merge(
    {
      "/var/app/docker-compose.yml"    = file("${path.module}/../../../docker-compose.yml")
      "/var/app/pgadmin4/servers.json" = file("${path.module}/../../../pgadmin4/servers.json")

      (local.env_file_path) = join("\n", [for k, v in local.env_vars : "${k}=${v}"])
    },
    {
      for source, target in local.templates :
      target => templatefile("${path.module}/templates/${source}", local.template_vars)
    }
  )

  files_by_perms = {
    "0444" = local.data_files
    "0555" = local.exec_files
  }

  template_vars = {
    env_file_path  = local.env_file_path
    server_os_user = local.server_os_user

    prometheus_remote_write_url = var.prometheus_remote_write_url
    prometheus_username         = var.prometheus_username
    prometheus_password         = var.prometheus_password

    loki_remote_write_url = "${var.loki_url}/loki/api/v1/push"
    loki_username         = var.loki_username
    loki_password         = var.loki_password
    loki_url              = var.loki_url

    hostname = local.hostname

    ssh_public_key = chomp(file("~/.ssh/id_rsa.pub"))
    server_os_user = local.server_os_user

    data_volume_device = hcloud_volume.master.linux_device
    data_volume_path   = local.data_volume_path
    data_volume_fs     = local.data_volume_fs

    docker_username = var.docker_username
    docker_password = var.docker_password

    workspace_kind = module.workspace.kind
  }

  env_vars = {
    LOKI_URL      = var.loki_url
    LOKI_USERNAME = var.loki_username
    LOKI_PASSWORD = var.loki_password

    PG_PASSWORD      = var.pg_password
    PGADMIN_PASSWORD = var.pgadmin_password

    PG_DATA          = local.pg_data
    DATA_VOLUME_PATH = local.data_volume_path

    TG_BOT_MEDIA_CACHE_CHAT = var.tg_bot_media_cache_chat
    TG_BOT_MAINTAINER       = var.tg_bot_maintainer
    TG_BOT_TOKEN            = var.tg_bot_token
    TG_BOT_IMAGE_NAME       = var.tg_bot_image_name
    TG_BOT_IMAGE_TAG        = var.tg_bot_image_tag
    TG_BOT_LOG              = join(",", local.log_filter)
    TG_BOT_LOG_LABELS = jsonencode({
      instance = local.hostname
    })

    DERPI_API_KEY = var.derpi_api_key
    DERPI_FILTER  = var.derpi_filter
  }
}

module "workspace" {
  source = "../workspace"
}

data "cloudinit_config" "master" {
  part {
    content = templatefile(
      "${path.module}/templates/user_data.yaml",
      merge(
        local.template_vars,
        {
          files = merge(
            flatten([
              for perms, files in local.files_by_perms : [
                for path, content in files : {
                  (path) = { content = base64gzip(content), perms = perms }
                }
              ]
            ])
            ...
          )
        }
      )
    )
  }
}

resource "hcloud_server" "master" {
  name        = local.hostname
  image       = "ubuntu-22.04"
  server_type = module.workspace.kind == "prod" ? "cpx21" : "cx11"
  location    = local.location
  user_data   = data.cloudinit_config.master.rendered

  public_net {
    # Not having IPv4 enabled reduces the cost, but we need it because we are
    # downloading some stuff from the public internet during the provisioning.
    ipv4_enabled = true
    ipv6_enabled = true
  }
}

resource "hcloud_volume" "master" {
  name     = "master${module.workspace.id_suffix}"
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

# # HACK: we need to gracefully shutdown our systemd service with the database
# # docker container before the data volume is detached. This null resource
# # depends on the volume attachment resource, so the remote-exec provisioner
# # teardown script will be run before the attachment is destroyed.
# #
# # Unfortunately, it's not possible to do this with `systemd`. The volume detach
# # sequence is undocumented in Hetzner docs. One would expect that all `systemd`
# # services dependent upon the volume's mount are stopped before the volume
# # is detached but this isn't true.
# #
# # The reality is cruel. It was experimentally found that the volume is
# # detached abruptly. Therefore the database doesn't have time to
# # flush its data to the disk, which means potential data loss.
resource "null_resource" "teardown" {
  triggers = {
    data_volume_attachment_id = hcloud_volume_attachment.master.id

    # The data volume attachment ID is enough for the trigger, but these
    # triggers are needed to workaround the problem that it's impossible
    # to reference symbols other than `self` variable in the provisioner block.
    #
    # Issue in terraform: https://github.com/hashicorp/terraform/issues/23679
    server_ip       = hcloud_server.master.ipv4_address
    server_os_user  = local.server_os_user
    systemd_service = local.systemd_service
  }

  provisioner "remote-exec" {
    when = destroy

    inline = [
      <<-SCRIPT
      #!/usr/bin/env bash
      set -euo pipefail
      sudo systemctl stop ${self.triggers.systemd_service} grafana-agent.service
      SCRIPT
    ]

    connection {
      host = self.triggers.server_ip
      user = self.triggers.server_os_user
    }
  }
}
