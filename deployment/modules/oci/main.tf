locals {
  env_vars = {
    LOKI_URL      = var.loki_url
    LOKI_USERNAME = var.loki_username
    LOKI_PASSWORD = var.loki_password

    PG_PASSWORD      = var.pg_password
    PGADMIN_PASSWORD = var.pgadmin_password

    PG_DATA                 = local.pg_data
    DATA_VOLUME_MOUNT_POINT = local.data_volume_mount_point

    TG_BOT_MAINTAINER = var.tg_bot_maintainer
    TG_BOT_TOKEN      = var.tg_bot_token
    TG_BOT_IMAGE_NAME = var.tg_bot_image_name
    TG_BOT_IMAGE_TAG  = var.tg_bot_image_tag
    TG_BOT_LOG = "debug,hyper=info,reqwest=info,rustls=info,sqlx=warn"
    TG_BOT_LOG_LABELS = jsonencode({
      instance = local.hostname
    })
  }

  ssh_public_key          = chomp(file("~/.ssh/id_rsa.pub"))
  hostname                = "oci-master${module.workspace.id_suffix}"
  data_volume_device      = "/dev/oracleoci/oraclevdb"
  data_volume_mount_point = "/mnt/master"
  data_volume_fs          = "ext4"

  # Looks like using the user name `admin` conflicts with something, because
  # the server is not accessible via SSH with this user name. The supposition
  # is that this conflicts with the `admin` group name already present in the
  # used linux AMI.
  server_os_user = "mane"

  pg_data = "${local.data_volume_mount_point}/data/postgres"

  env_file_path = "/var/app/.env"

  systemd_service = "tg-bot.service"

  template_vars = {
    env_file_path  = local.env_file_path
    server_os_user = local.server_os_user

    prometheus_remote_write_url = var.prometheus_remote_write_url
    prometheus_username         = var.prometheus_username
    prometheus_password         = var.prometheus_password

    loki_remote_write_url = "${var.loki_url}/loki/api/v1/push"
    loki_username         = var.loki_username
    loki_password         = var.loki_password
    loki_url              = var.loki_url

    hostname = local.hostname

    ssh_public_key = local.ssh_public_key
    server_os_user = local.server_os_user

    data_volume_device      = local.data_volume_device
    data_volume_mount_point = local.data_volume_mount_point
    data_volume_fs          = local.data_volume_fs

    docker_username = var.docker_username
    docker_password = var.docker_password

    // Reboot only when running in production. It usually isn't important,
    // so we can skip this step in development.
    reboot_if_required = module.workspace.kind == "prod"
  }

  templates = {
    "grafana-agent.yaml"    = "/etc/grafana-agent.yaml"
    (local.systemd_service) = "/etc/systemd/system/tg-bot.service"
  }

  exec_files = {
    "/var/app/docker-compose.sh" = file("${path.module}/templates/docker-compose.sh")
    "/var/app/start.sh"          = file("${path.module}/templates/start.sh")
  }

  data_files = merge(
    {
      "/var/app/docker-compose.yml"    = file("${path.module}/../../../docker-compose.yml")
      "/var/app/pgadmin4/servers.json" = file("${path.module}/../../../pgadmin4/servers.json")

      (local.env_file_path) = join("\n", [for k, v in local.env_vars : "${k}=${v}"])
    },
    {
      for source, target in local.templates :
      target => templatefile("${path.module}/templates/${source}", local.template_vars)
    }
  )

  files_by_perms = {
    "0444" = local.data_files
    "0555" = local.exec_files
  }

  compartment_id      = oci_identity_compartment.master.id
  display_name        = "only-hooves-tg-bot${module.workspace.id_suffix}"
  availability_domain = data.oci_identity_availability_domains.master.availability_domains[1].name

  # Our tagret is to fit into the Always Free capacity of Oracle Cloud.
  # We could use the full capacity for the production instance, but we
  # also reserve some amount of that for development and testing.
  capacity_by_workspace = {
    ram_gbs         = { prod = 18, dev = 6 }
    ocpus           = { prod = 3, dev = 1 }
    boot_volume_gbs = { prod = 50, dev = 50 }
    data_volume_gbs = { prod = 50, dev = 50 }
  }
  capacity = {
    for key, val in local.capacity_by_workspace : key => val[module.workspace.kind]
  }
}

module "workspace" {
  source = "../workspace"
}

data "cloudinit_config" "master" {
  part {
    content = templatefile(
      "${path.module}/templates/user_data.yaml",
      merge(
        local.template_vars,
        {
          files = merge(
            flatten([
              for perms, files in local.files_by_perms : [
                for path, content in files : {
                  (path) = { content = base64gzip(content), perms = perms }
                }
              ]
            ])
            ...
          )
        }
      )
    )
  }
}

data "oci_core_images" "master" {
  compartment_id           = local.compartment_id
  operating_system         = "Canonical Ubuntu"
  operating_system_version = "22.04"
  state                    = "AVAILABLE"
  shape                    = "VM.Standard.A1.Flex"
  sort_by                  = "TIMECREATED"
  sort_order               = "DESC"
}

data "oci_identity_availability_domains" "master" {
  compartment_id = local.compartment_id
}

resource "oci_identity_compartment" "master" {
  name           = local.display_name
  compartment_id = var.parent_compartment_id
  description    = "OnlyHooves Telegram bot (${module.workspace.kind})"
  enable_delete  = true
}

resource "oci_core_instance" "master" {
  display_name        = local.display_name
  compartment_id      = local.compartment_id
  availability_domain = local.availability_domain
  shape               = "VM.Standard.A1.Flex"

  # We don't store persistent data on the boot volume and use a separate one
  # for that instead to have reproducible deployments.
  preserve_boot_volume = false

  metadata = {
    ssh_authorized_keys = local.ssh_public_key
    user_data           = sensitive(data.cloudinit_config.master.rendered)
  }

  shape_config {
    memory_in_gbs = local.capacity.ram_gbs
    ocpus         = local.capacity.ocpus
  }

  source_details {
    source_id               = data.oci_core_images.master.images[0].id
    boot_volume_size_in_gbs = local.capacity.boot_volume_gbs
    source_type             = "image"
  }

  create_vnic_details {
    assign_public_ip = true
    hostname_label   = local.hostname
    subnet_id        = oci_core_subnet.master.id
  }

  instance_options {
    are_legacy_imds_endpoints_disabled = true
  }

  depends_on = [
    oci_core_volume.master_data,
    oci_core_route_table.master,
  ]
}

resource "oci_core_volume" "master_data" {
  display_name        = local.display_name
  compartment_id      = local.compartment_id
  availability_domain = local.availability_domain
  size_in_gbs         = local.capacity.data_volume_gbs
}

resource "oci_core_volume_attachment" "master_data" {
  display_name    = local.display_name
  instance_id     = oci_core_instance.master.id
  volume_id       = oci_core_volume.master_data.id
  device          = local.data_volume_device
  attachment_type = "paravirtualized"
}

# HACK: we need to gracefully shutdown our systemd service with the database
# docker container before the data volume is detached. This null resource
# depends on the volume attachment resource, so the remote-exec provisioner
# teardown script will be run before the attachment is destroyed.
#
# Unfortunately, it's not possible to do this with `systemd`. The volume detach
# sequence is undocumented in OCI docs. One would expect that all `systemd`
# services dependent upon the volume's mount are stopped before the volume
# is detached but this isn't true.
#
# The reality is cruel. It was experimentally found that the volume is
# detached abruptly. Therefore the database doesn't have time to
# flush its data to the disk, which means potential data loss.
resource "null_resource" "teardown" {
  triggers = {
    data_volume_attachment_id = oci_core_volume_attachment.master_data.id

    # The data volume attachment ID is enough for the trigger, but these
    # triggers are needed to workaround the problem that it's impossible
    # to reference symbols other than `self` variable in the provisioner block.
    #
    # Issue in terraform: https://github.com/hashicorp/terraform/issues/23679
    server_ip       = oci_core_instance.master.public_ip
    server_os_user  = local.server_os_user
    systemd_service = local.systemd_service
  }

  provisioner "remote-exec" {
    when = destroy

    inline = [
      <<-SCRIPT
      #!/usr/bin/env bash
      set -euo pipefail
      sudo systemctl stop ${self.triggers.systemd_service}
      SCRIPT
    ]

    connection {
      host = self.triggers.server_ip
      user = self.triggers.server_os_user
    }
  }
}

# ------------------------------------------------
# ------------------ Networking ------------------
# ------------------------------------------------

resource "oci_core_vcn" "master" {
  display_name   = local.display_name
  compartment_id = local.compartment_id
  dns_label      = "master"
  cidr_block     = "172.16.0.0/20"
}

resource "oci_core_subnet" "master" {
  compartment_id = local.compartment_id
  display_name   = local.display_name
  vcn_id         = oci_core_vcn.master.id
  route_table_id = oci_core_route_table.master.id
  cidr_block     = "172.16.0.0/24"
  dns_label      = "master"
}

resource "oci_core_route_table" "master" {
  display_name   = local.display_name
  compartment_id = local.compartment_id
  vcn_id         = oci_core_vcn.master.id
  route_rules {
    network_entity_id = oci_core_internet_gateway.master.id
    destination_type  = "CIDR_BLOCK"
    destination       = "0.0.0.0/0"
  }
}

resource "oci_core_internet_gateway" "master" {
  display_name   = local.display_name
  compartment_id = local.compartment_id
  vcn_id         = oci_core_vcn.master.id
}
