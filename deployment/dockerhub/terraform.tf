terraform {
  required_providers {
    dockerhub = {
      source  = "BarnabyShearer/dockerhub"
      version = "~> 0.0.8"
    }
  }
  required_version = ">= 1.2"
}
