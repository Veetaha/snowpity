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

variable "loki_remote_write_url" {
  type = string
}

variable "loki_username" {
  type = string
}

variable "loki_password" {
  type      = string
  sensitive = true
}
