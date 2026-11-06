locals {
  hcloud_token           = var.hcloud_token[module.workspace.kind]
  tg_bot_alerts_chat     = var.tg_bot_alerts_chat[module.workspace.kind]
  tg_bot_blob_cache_chat = var.tg_bot_blob_cache_chat[module.workspace.kind]
  tg_bot_token           = var.tg_bot_token[module.workspace.kind]
}

module "workspace" {
  source = "../modules/workspace"
}

module "hetzner" {
  source = "../modules/hetzner"

  tg_bot_alerts_chat     = local.tg_bot_alerts_chat
  tg_bot_image_name      = var.tg_bot_image_name
  tg_bot_image_tag       = var.tg_bot_image_tag
  tg_bot_maintainer      = var.tg_bot_maintainer
  tg_bot_blob_cache_chat = local.tg_bot_blob_cache_chat
  tg_bot_token           = local.tg_bot_token

  twitter_cookies = var.twitter_cookies

  pg_password      = var.pg_password
  pgadmin_password = var.pgadmin_password

  hcloud_token    = local.hcloud_token
  allowed_ssh_ips = var.allowed_ssh_ips
  ssh_public_keys = var.ssh_public_keys
}
