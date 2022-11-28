variable "veebot_tg_image_tag" {
  nullable = false
  type     = string
  default  = "latest"
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

variable "grafana_cloud_api_key" {
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

variable "oci_parent_compartment_id" {
  nullable = false
  type     = string
}
