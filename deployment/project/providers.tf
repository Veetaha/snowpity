provider "dockerhub" {
  username = var.docker_username
  password = var.docker_password
}

terraform {
  required_version = ">= 1.2"

  required_providers {
    oci = {
      source  = "oracle/oci"
      version = "~> 4.100.0"
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
