locals {
  tg_bot_token            = var.tg_bot_token[module.workspace.kind]
  tg_bot_media_cache_chat = var.tg_bot_media_cache_chat[module.workspace.kind]
  hcloud_token            = var.hcloud_token[module.workspace.kind]
}

module "workspace" {
  source = "../modules/workspace"
}

module "hetzner" {
  source = "../modules/hetzner"

  tg_bot_media_cache_chat = local.tg_bot_media_cache_chat
  tg_bot_maintainer       = var.tg_bot_maintainer
  tg_bot_token            = local.tg_bot_token
  tg_bot_image_tag        = var.tg_bot_image_tag
  tg_bot_image_name       = module.dockerhub.image_name

  pg_password      = var.pg_password
  pgadmin_password = var.pgadmin_password

  docker_username = var.docker_username
  docker_password = var.docker_password

  derpi_api_key = var.derpi_api_key
  derpi_filter  = var.derpi_filter

  allowed_ssh_ips = var.allowed_ssh_ips
}

module "dockerhub" {
  source = "../modules/dockerhub"
}
