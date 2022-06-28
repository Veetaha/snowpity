# User DOCKER_USERNAME, DOCKER_PASSWORD for auth
provider "dockerhub" {}

resource "dockerhub_repository" "veebot_telegram" {
  namespace        = "veetaha"
  name             = "veebot-telegram"
  description      = "Telegram bot with assorted functionality"
  full_description = "More information in [Github repository](https://github.com/Veetaha/veebot-telegram)"
  private          = false
}
