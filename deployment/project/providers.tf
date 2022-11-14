provider "hcloud" {
  token = local.hcloud_token
}

provider "dockerhub" {
  username = var.docker_username
  password = var.docker_password
}

terraform {
  required_version = ">= 1.2"

  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.35.2"
    }

    cloudinit = {
      source  = "hashicorp/cloudinit"
      version = "~> 2.2.0"
    }

    grafana = {
      source  = "grafana/grafana"
      version = "~> 1.30.0"
    }

    dockerhub = {
      source  = "BarnabyShearer/dockerhub"
      version = "~> 0.0.8"
    }
  }
}
