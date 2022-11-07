output "id_suffix" {
  description = <<EOF
    Token that can be inserted into the resource id/name suffix to make it different from other workspace.

    The workspace is selected based on `terraform.workspace` value. The `default` workspace is
    the production workspace.

    Beware that it will be empty for production workspace.
  EOF

  value = terraform.workspace == "default" ? "" : "-dev"
}

output "kind" {
  description = "Discrimnator to take apart real production workspace and non-production workspaces"
  value       = terraform.workspace == "default" ? "prod" : "dev"
}
