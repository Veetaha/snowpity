locals {
  veebot_tg_env_vars = {
    VEEBOT_LOG = "debug,hyper=info,reqwest=info,rustls=info,sqlx=warn"

    TG_BOT_MAINTAINER = var.tg_bot_maintainer
    TG_BOT_TOKEN      = var.tg_bot_token

    LOKI_URL      = var.loki_url
    LOKI_USERNAME = var.loki_username
    LOKI_PASSWORD = var.loki_password

    PG_PASSWORD      = var.pg_password
    PGADMIN_PASSWORD = var.pgadmin_password

    PG_DATA = local.pg_data

    VEEBOT_TG_IMAGE_NAME = var.veebot_tg_image_name
    VEEBOT_TG_IMAGE_TAG  = var.veebot_tg_image_tag
    VEEBOT_LOG_LABELS = jsonencode({
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

  templates = {
    "grafana-agent.yaml" = {
      target = "/etc/grafana-agent.yaml"
      vars = {
        prometheus_remote_write_url = var.prometheus_remote_write_url
        prometheus_username         = var.prometheus_username
        prometheus_password         = var.prometheus_password

        loki_remote_write_url = "${var.loki_url}/loki/api/v1/push"
        loki_username         = var.loki_username
        loki_password         = var.loki_password

        hostname = local.hostname
      }
    },
    "veebot-tg.service" = {
      target = "/etc/systemd/system/veebot-tg.service"
      vars = {
        docker_compose_cmd = "/usr/bin/env bash /var/app/docker-compose.sh"
        env_file_path      = local.env_file_path
        server_os_user     = local.server_os_user
      }
    }
    "docker-compose.sh" = {
      target = "/var/app/docker-compose.sh"
      vars   = {}
    }
  }

  non_templates = {
    "/var/app/docker-compose.yml"    = file("${path.module}/../../../docker-compose.yml"),
    "/var/app/pgadmin4/servers.json" = file("${path.module}/../../../pgadmin4/servers.json"),

    "${local.env_file_path}" = join("\n", [for k, v in local.veebot_tg_env_vars : "${k}=${v}"]),
  }

  files = merge(
    {
      for template_source, template in local.templates : template.target => templatefile(
        "${path.module}/templates/${template_source}", template.vars
      )
    },
    local.non_templates
  )

  user_data_vars = {
    files          = { for path, content in local.files : path => base64gzip(content) }
    ssh_public_key = local.ssh_public_key
    server_os_user = local.server_os_user

    data_volume_device      = local.data_volume_device
    data_volume_mount_point = local.data_volume_mount_point
    data_volume_fs          = local.data_volume_fs

    loki_url      = var.loki_url
    loki_username = var.loki_username
    loki_password = var.loki_password

    docker_username = var.docker_username
    docker_password = var.docker_password

    // Reboot only when running in production. It usually isn't important,
    // so we can skip this step in development.
    reboot_if_required = module.workspace.kind == "prod"
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
    content = templatefile("${path.module}/templates/user_data.yaml", local.user_data_vars)
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
