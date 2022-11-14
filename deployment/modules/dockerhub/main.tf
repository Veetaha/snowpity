resource "dockerhub_repository" "veebot_telegram" {
  namespace        = "veetaha"
  name             = "veebot-telegram${module.workspace.id_suffix}"
  description      = "Telegram bot with assorted functionality"
  full_description = "More information in [Github repository](https://github.com/Veetaha/veebot-telegram)"
  private          = module.workspace.kind == "dev"
}

module "workspace" {
  source = "../workspace"
}

output "image_name" {
  value = "${dockerhub_repository.veebot_telegram.namespace}/${dockerhub_repository.veebot_telegram.name}"
}
