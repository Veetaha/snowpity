variable "tg_bot_alerts_chat" {
  nullable = false
  type     = string
}

variable "tg_bot_blob_cache_chat" {
  nullable = false
  type     = string
}

variable "tg_bot_maintainer" {
  nullable = false
  type     = string
}

variable "tg_bot_image_tag" {
  nullable = false
  type     = string
}

variable "tg_bot_image_name" {
  nullable = false
  type     = string
}

variable "tg_bot_token" {
  nullable  = false
  type      = string
  sensitive = true
}

variable "allowed_ssh_ips" {
  nullable  = false
  type      = list(string)
  sensitive = true
}

variable "pg_password" {
  nullable  = false
  type      = string
  sensitive = true
}

variable "pgadmin_password" {
  nullable  = false
  type      = string
  sensitive = true
}

variable "docker_username" {
  nullable  = false
  sensitive = true
  type      = string
}

variable "docker_password" {
  nullable  = false
  sensitive = true
  type      = string
}

variable "twitter_cookies" {
  nullable  = false
  sensitive = true
  type      = string
}
