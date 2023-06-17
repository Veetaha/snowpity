# Logging driver

There is a huge caveat in docker, that it stores container logs in an always growing JSON file without truncating it. It may lead to disk exhaustion.

We are overriding the default one with the "local" logging driver and enabled retention configs.

More info can be found in [the docs](https://docs.docker.com/config/containers/logging/configure/#configure-the-default-logging-driver).
