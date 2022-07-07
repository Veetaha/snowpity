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
