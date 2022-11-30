variable "parent_compartment_id" {
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

variable "prometheus_remote_write_url" {
  nullable = false
  type     = string
}

variable "prometheus_username" {
  nullable = false
  type     = string
}

variable "prometheus_password" {
  nullable  = false
  type      = string
  sensitive = true
}

variable "loki_url" {
  nullable = false
  type     = string
}

variable "loki_username" {
  nullable = false
  type     = string
}

variable "loki_password" {
  nullable  = false
  type      = string
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
