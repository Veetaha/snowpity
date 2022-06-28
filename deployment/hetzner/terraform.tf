terraform {
  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.33.2"
    }
  }
  required_version = ">= 1.2"
}
