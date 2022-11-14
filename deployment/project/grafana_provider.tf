# Grafana Cloud is quite special in its provider configuration.
# It requires two providers: one to interact with the global Grafana Cloud API
# and the second one to interact with the specific Grafana Cloud Stack.
#
# Because this setup requires creating intermediate resources and just takes
# considerable amount of code, it lives separately from general `providers.tf`

provider "grafana" {
  alias         = "cloud"
  cloud_api_key = local.grafana_cloud_api_key
}

provider "grafana" {
  alias = "cloud_stack"

  url  = grafana_cloud_stack.this.url
  auth = grafana_api_key.deployment.key
}

resource "grafana_cloud_stack" "this" {
  provider = grafana.cloud

  name        = "veebot-telegram"
  slug        = "vtg${module.workspace.id_suffix_alnum}"
  region_slug = "eu"
}

resource "grafana_api_key" "deployment" {
  provider = grafana.cloud

  cloud_stack_slug = grafana_cloud_stack.this.slug
  name             = "veebot-telegram-deployment-key"
  role             = "Admin"
}
