[rust-toolchain]: https://www.rust-lang.org/tools/install

# veebot-telegram

This is a Telegram bot for me and friends.
It has assorted functionality:

- Verify the chat for the presence of messages with banned patterns in them


# Development

To build the bot from sources, we need to have [Rust toolchain installed][rust-toolchain].

To build and run the bot in development mode with this:

```bash
cargo run
```

# Configuration

The bot is configured via the environment variables.
Env variables can also be specified in `.env` file.
See [`EXAMPLE.env`](EXAMPLE.env) for example and documentation of the config.
