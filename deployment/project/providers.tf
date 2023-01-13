provider "hcloud" {
  token = local.hcloud_token
}

provider "dockerhub" {
  username = var.docker_username
  password = var.docker_password
}

terraform {
  # Make sure to keep it in sync with the version requirement on CI
  required_version = ">= 1.3"

  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.36.1"
    }

    cloudinit = {
      source  = "hashicorp/cloudinit"
      version = "~> 2.2.0"
    }

    grafana = {
      source  = "grafana/grafana"
      version = "~> 1.31.1"
    }

    dockerhub = {
      source  = "BarnabyShearer/dockerhub"
      version = "~> 0.0.15"
    }
  }
}
