[rust-toolchain]: https://www.rust-lang.org/tools/install

# veebot-telegram

This is a Telegram bot for me and friends.
It has assorted functionality for managing our Telegram chat.


# Development

To build the bot from sources, there has to be [Rust toolchain installed][rust-toolchain].

To build and run the bot in development mode run this:

```bash
cargo run
```

# Configuration

The bot is configured via the environment variables.
The environment variables can also be specified in a `.env` file.
See [`EXAMPLE.env`](EXAMPLE.env) as an example with documentation of the config.
