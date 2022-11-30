resource "dockerhub_repository" "snowpity_tg" {
  namespace        = "veetaha"
  name             = "snowpity-tg${module.workspace.id_suffix}"
  description      = "Telegram bot with assorted functionality"
  full_description = "More information in [Github repository](https://github.com/Veetaha/snowpity)"
  private          = module.workspace.kind == "dev"
}

module "workspace" {
  source = "../workspace"
}

output "image_name" {
  value = "${dockerhub_repository.snowpity_tg.namespace}/${dockerhub_repository.snowpity_tg.name}"
}
