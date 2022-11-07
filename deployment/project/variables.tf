variable "veebot_tg_image_tag" {
  type    = string
  default = "latest"
}

variable "tg_bot_token" {
  sensitive = true
  type      = string
}

variable "hcloud_token" {
  sensitive = true
  type      = string
}

variable "grafana_cloud_api_key" {
  sensitive = true
  type      = string
}

variable "docker_username" {
  sensitive = true
  type      = string
}

variable "docker_password" {
  sensitive = true
  type      = string
}

variable "pg_password" {
  type      = string
  sensitive = true
}

variable "pgadmin_password" {
  type      = string
  sensitive = true
}
