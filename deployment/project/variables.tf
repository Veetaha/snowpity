variable "tg_bot_alerts_chat" {
  nullable = false
  type = object({
    prod = optional(string)
    dev  = optional(string)
  })
}

variable "tg_bot_image_tag" {
  nullable = false
  type     = string
}

variable "tg_bot_media_cache_chat" {
  nullable = false
  type = object({
    prod = optional(string)
    dev  = optional(string)
  })
}

variable "tg_bot_maintainer" {
  nullable = false
  type     = string
}

variable "tg_bot_token" {
  nullable  = false
  sensitive = true
  type = object({
    prod = optional(string)
    dev  = optional(string)
  })
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

variable "hcloud_token" {
  nullable  = false
  sensitive = true
  type = object({
    prod = optional(string)
    dev  = optional(string)
  })
}

variable "allowed_ssh_ips" {
  nullable  = false
  type      = list(string)
  sensitive = true
}

variable "twitter_bearer_token" {
  nullable  = false
  sensitive = true
  type      = string
}
