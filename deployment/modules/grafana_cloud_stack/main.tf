locals {
  dashboards = "${path.module}/dashboards"
}

resource "grafana_folder" "linux_node" {
  title = "linux-node"
}

resource "grafana_dashboard" "node_exporter_nodes" {
  folder      = grafana_folder.linux_node.id
  config_json = file("${local.dashboards}/node_exporter/nodes.json")
}

resource "grafana_dashboard" "node_exporter_use_method_node" {
  folder      = grafana_folder.linux_node.id
  config_json = file("${local.dashboards}/node_exporter/use_method_node.json")
}
