# Unreleased

# 0.7.0

## Added

- Added support for sharing media from Twitter 🎉
- Added an ability to write custom comments under the posted media.
  The comments are the text that follows the link and a new line

### Internal

- Added `--tag` option for deployment to automatically push git tags after the build
- Added victoria metrics to forwarded SSH ports. Turns out it's difficult to remember
  to list all necessary ports for forwarding in SSH 😅 (SSH is used only for administration)

## Changed

- Improved error handling. Now errors are define on-per module basis instead of
  a single big app-wide error module.

## Removed

### Internal

- Derpibooru API key was not used, and in fact it is not needed at all for readonly operations. Also removed unnecessary `derpi_filter` config, which wasn't used.

# 0.6.0

## Added

- Added support for sharing videos from derpibooru

## Changed

- Improved the help message. Now it includes the reference animation of how to use bot in inline mode
- Implemented better support for GIFs by using their soundless MP4 representation
- Now we are using `/images/{id}` in the links of derpibooru, because bare id in the path seems like an API that derpibooru might deprecate and remove in the future
- Added better error handling in inline queries. Now they send an error result if an error happens.

# 0.5.1

## Added

- The help message of the bot now displays example of the usage of inline query feature

## Fixed

- Now links to users prefer `t.me` URL, because links via user IDs may not work if the user restricted "Forward Messages" in their privacy settings
- The `md_link()` now displays the full name instead of preferring user tag, because user tag is used in the `t.me` link, so this way the link contains more info about the user (full name and user tag)
- Disable `/ftai` command for now, because the service is unavailable, and we don't want to throw errors at users

# 0.5.0

## Added

- Added derpibooru integration via telegram bot inline queries. Now media is forwarded to telegram cache chat and then to the inline queries response
- Added `/chat_config` (owner only) displays the configuration of the telegram chat
- Added `/toggle_captcha` (owner only) disables or enables the captcha verification in the telegram chat
- Added logging to owner commands to keep track of non-owner users trying to use them
- Added metrics instrumentation where possible
- Added `metric-bat/metrics-bat-macros` crates with the missing batteries for `metrics` crates
- Added `sqlx-bat` crate with the missing batteries for `sqlx` and `sea-query` crates
- Added `snowpity-tg-macros` crate with the ad hoc proc macros for `snowpity-tg`
- Added Postgres database to the stack to implement several stateful features. Now the our data volume is going to be used, so we must be very careful with it, and use migrations not to lose our state data.
- Added span trace capturing to errors


## Changed

- Renamed the application to "Snowpity" branding
- Renamed `/admin_help` to `owner_help`. All admin commands are now accessible only to the owner
- Migrated from Grafana Cloud to self-hosted `grafana`, `loki` and `victoria-metrics` in `docker-compose`
- Migrated to `seaorm` and then back to `sqlx`, so almost nothing changed (lol)
- Migrated to Oracle Cloud and then back to Hetzner, so almost nothing changed (lol)
- Migrated most of the automation from bash and xtask to `x.nu` (nushell)
- Migrated to the latest and greates Rust `1.66.0`

## Fixed

- The bot now clears the unverified users map for the chat when it was kicked from one

## Removed

- Removed the `censy` crate and other code in `snowpity-tg` leftover from the attempts to implement swearwords and general censorship in the telegram chat. Now this feature is either postponed, or won't ever be implemented.

# 0.4.3

## Added

- `/sys` maintainer command displays the current diagnostic system information
- `/list_unverified` maintainer command displays the list of users in unverified users map
- `/clear_unverified` clears the unverified users map and cancels captcha timeouts
- Different `prod` and `dev` deployments workspaces are now fully supported and working correctly.
  It is now possible to deploy the entire dockerhub repo / grafana stack / hetzner infrastructure
  independently for production and development end-to-end testing purposes.

## Changed

- Improved the debug representation of `Chat` and `User` objects by using their ultimate telegram links. It is now possible to just click the link in the logs to open the chat or user profile in telegram.

## Fixed

- Fixed remaining bugs in captcha. The bot now saves the information about the user rights when they joined the chat and restrores them when captcha finishes. The bot doesn't restore the original permissions when the user's rights were changed during the captcha process by some other admin.


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
