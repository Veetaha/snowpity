locals {
  veebot_tg_env_vars = {
    VEEBOT_LOG = "debug,hyper=info,reqwest=info,rustls=info,sqlx=warn"

    TG_BOT_MAINTAINER = "326348880"
    TG_BOT_TOKEN      = var.tg_bot_token

    LOKI_URL      = var.loki_url
    LOKI_USERNAME = var.loki_username
    LOKI_PASSWORD = var.loki_password

    PG_PASSWORD      = var.pg_password
    PGADMIN_PASSWORD = var.pgadmin_password

    PGDATA = local.pg_data

    VEEBOT_TG_IMAGE_TAG = var.veebot_tg_image_tag
    VEEBOT_LOG_LABELS = jsonencode({
      instance = local.hostname
    })
  }

  location           = "fsn1"
  ssh_public_key     = file("~/.ssh/id_rsa.pub")
  hostname           = "hetzner-master${module.workspace.id_suffix}"
  volume_mount_point = "/mnt/master"
  volume_fs          = "ext4"

  pg_data = "${local.volume_mount_point}/data/postgres"

  env_file_path = "/var/app/.env"

  templates = {
    "grafana-agent.yaml" = {
      target = "/etc/grafana-agent.yaml"
      vars = {
        prometheus_remote_write_url = var.prometheus_remote_write_url
        prometheus_username         = var.prometheus_username
        prometheus_password         = var.prometheus_password

        loki_remote_write_url = "${var.loki_url}/loki/api/v1/push"
        loki_username         = var.loki_username
        loki_password         = var.loki_password

        hostname = local.hostname
      }
    },
    "veebot-tg.service" = {
      target = "/etc/systemd/system/veebot-tg.service"
      vars = {
        docker_compose_cmd = "/usr/bin/env bash /var/app/docker-compose.sh"
        env_file_path      = local.env_file_path
      }
    }
    "docker-compose.sh" = {
      target = "/var/app/docker-compose.sh"
      vars   = {}
    }
  }

  non_templates = {
    "/var/app/docker-compose.yml" = file("${path.module}/../../../docker-compose.yml"),
    "/var/app/pgadmin4/servers.json" = file("${path.module}/../../../pgadmin4/servers.json"),

    "${local.env_file_path}" = join("\n", [for k, v in local.veebot_tg_env_vars : "${k}=${v}"]),
  }

  files = merge(
    {
      for template_source, template in local.templates : template.target => templatefile(
        "${path.module}/templates/${template_source}", template.vars
      )
    },
    local.non_templates
  )

  user_data_vars = {
    files          = { for path, content in local.files : path => base64gzip(content) }
    ssh_public_key = local.ssh_public_key
    pgdata         = local.pg_data

    volume_device      = hcloud_volume.master.linux_device
    volume_mount_point = local.volume_mount_point
    volume_fs          = local.volume_fs

    loki_url      = var.loki_url
    loki_username = var.loki_username
    loki_password = var.loki_password
  }
}

module "workspace" {
  source = "../workspace"
}

data "cloudinit_config" "master" {
  part {
    content = templatefile("${path.module}/templates/user_data.yaml", local.user_data_vars)
  }
}

resource "hcloud_server" "master" {
  name        = local.hostname
  image       = "ubuntu-22.04"
  server_type = module.workspace.kind == "prod" ? "cpx21" : "cx11"
  location    = local.location
  ssh_keys    = [hcloud_ssh_key.admin.id]
  user_data   = data.cloudinit_config.master.rendered

  public_net {
    # Not having IPv4 enabled reduces the cost
    ipv4_enabled = false
    ipv6_enabled = true
  }
}

resource "hcloud_ssh_key" "admin" {
  name       = "admin${module.workspace.id_suffix}"
  public_key = local.ssh_public_key
}

resource "hcloud_volume" "master" {
  name     = "master${module.workspace.id_suffix}"
  size     = module.workspace.kind == "prod" ? 50 : 10
  location = local.location
}

resource "hcloud_volume_attachment" "master" {
  server_id = hcloud_server.master.id
  volume_id = hcloud_volume.master.id

  # automount doesn't work if server's cloud-init script contains `runcmd` module
  # <https://github.com/hetznercloud/terraform-provider-hcloud/issues/473#issuecomment-971535629>
  automount = false
}
