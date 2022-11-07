variable "veebot_tg_image_tag" {
  type = string
}

variable "tg_bot_token" {
  type      = string
  sensitive = true
}

variable "prometheus_remote_write_url" {
  type = string
}

variable "prometheus_username" {
  type = string
}

variable "prometheus_password" {
  type      = string
  sensitive = true
}

variable "loki_url" {
  type = string
}

variable "loki_username" {
  type = string
}

variable "loki_password" {
  type      = string
  sensitive = true
}

variable "pg_password" {
  type      = string
  sensitive = true
}

variable "pgadmin_password" {
  type      = string
  sensitive = true
}
