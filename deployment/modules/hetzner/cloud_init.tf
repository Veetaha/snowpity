locals {
  log_filter = [
    "debug",
    "hyper=info",
    "reqwest=info",
    "rustls=info",
    "sqlx=warn",
    "h2=info",
    "teloxide_core::adaptors::throttle::worker=info",
  ]

  data_volume_path = "/mnt/master"
  data_volume_fs   = "ext4"
  pg_data          = "${local.data_volume_path}/data/postgres"
  env_file_path    = "/var/app/.env"
  repo             = "${path.module}/../../.."
  bootstrap        = "${path.module}/bootstrap"

  template_files = {
    "tg-bot.service"      = "/etc/systemd/system/tg-bot.service"
    "data-volume.service" = "/etc/systemd/system/data-volume.service"
    "docker-daemon.json"  = "/etc/docker/daemon.json"
  }
  data_files = merge(
    {
      "/var/app/docker-compose.yml" = file("${local.repo}/docker-compose.yml")
      (local.env_file_path)         = join("\n", [for k, v in local.env_vars : "${k}=${v}"])
    },
    {
      for provisioning_file in fileset("${local.repo}", "docker/provisioning/**") :
      "/var/app/${provisioning_file}" => file("${local.repo}/${provisioning_file}")
    },
    {
      for source, target in local.template_files :
      target => templatefile("${local.bootstrap}/${source}", local.template_vars)
    }
  )

  exec_files = {
    for file in fileset(local.bootstrap, "*.sh") :
    "/var/app/${file}" => file("${local.bootstrap}/${file}")
  }

  files_by_perms = {
    "0444" = local.data_files
    "0555" = local.exec_files
  }

  template_vars = {
    env_file_path  = local.env_file_path
    server_os_user = local.server_os_user

    ssh_public_key = chomp(file("~/.ssh/id_rsa.pub"))

    data_volume_device = hcloud_volume.master.linux_device
    data_volume_path   = local.data_volume_path
    data_volume_fs     = local.data_volume_fs

    docker_username = var.docker_username
    docker_password = var.docker_password

    workspace_kind = module.workspace.kind
  }

  env_vars = {
    PG_PASSWORD      = var.pg_password
    PGADMIN_PASSWORD = var.pgadmin_password

    PG_DATA          = local.pg_data
    DATA_VOLUME_PATH = local.data_volume_path

    TG_BOT_ALERTS_CHAT      = var.tg_bot_alerts_chat
    TG_BOT_IMAGE_NAME       = var.tg_bot_image_name
    TG_BOT_IMAGE_TAG        = var.tg_bot_image_tag
    TG_BOT_MAINTAINER       = var.tg_bot_maintainer
    TG_BOT_MEDIA_CACHE_CHAT = var.tg_bot_media_cache_chat
    TG_BOT_TOKEN            = var.tg_bot_token
    TG_BOT_LOG              = join(",", local.log_filter)
    TG_BOT_LOG_LABELS = jsonencode({
      instance = local.hostname
    })
  }
}

data "cloudinit_config" "master" {
  part {
    content = templatefile(
      "${path.module}/bootstrap/user_data.yml",
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
