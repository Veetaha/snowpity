# 0.4.3

# Added

- `/sys` maintainer command displays the current diagnostic system information
- `/list_unverified` maintainer command displays the list of users in unverified users map
- `/clear_unverified` clears the unverified users map and cancels captcha timeouts
- Different `prod` and `dev` deployments workspaces are now fully supported and working correctly.
  It is now possible to deploy the entire dockerhub repo / grafana stack / hetzner infrastructure
  independently for production and development end-to-end testing purposes.

# Changed

- Improved the debug representation of `Chat` and `User` objects by using their ultimate telegram links. It is now possible to just click the link in the logs to open the chat or user profile in telegram.

# Fixed

- Fixed remaining bugs in captcha. The bot now save the information about the user rights when they joined the chat and restrores them when captcha finishes. The bot doesn't restore the original permissions when the user's rights were changed during the captcha process by some other admin.


# 0.4.2

## Changed

- Updated `teloxide` from `0.10` to `0.11`. This should fix some parsing bugs
- Improved logging in captcha module
- Made the teardown in development mode instant by skipping teloxide's shutdown logic
- Waiting for tracing-loki to flush logs for 3 seconds before teardown heuristically in release mode
- Changed the deployment folder to use terraform workspace for production and development
- Started working on the ingretaion of the database and censoring logic (not finished yet)
- Migrated to Rust 1.65.0
- Added maintainer `/describe` command for debugging of the messages and their senders

# 0.4.1

## Changed

- Updated captcha question to a more straightforward one without swearwords

# 0.4.0

## Changed

- Move the bot to Hetzner Cloud
- Scrape the server with grafana-agent sending telemetry to Grafana Cloud
    - The Linux server is configured with node_exporter only
- Add a script to SSH to Hetzner server
- Add a script to deploy the bot on Hetzner server and upgrade the docker image
- Define full infrastructure with terraform that include the deployment of the following:
    - Dockerhub repository
    - Grafana Cloud Stack
    - Hetzner server
- Add .editorconfig
- Add `cargo xtask` utility for development in docker-compose environment
- Add `tracing_loki` layer to the applicating to send logs to Grafana Cloud
- Add a NAS volume for Hetzner server to use as persistent storage for future Postgres instance
