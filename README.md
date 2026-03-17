# agentgif (Rust)

CLI for [AgentGIF](https://agentgif.com) — upload, manage, and share terminal GIFs.

Built with [clap](https://crates.io/crates/clap) + [reqwest](https://crates.io/crates/reqwest).

## Install

```bash
cargo install agentgif
```

Requires Rust 1.70+.

## Quick Start

```bash
# Authenticate via browser
agentgif login

# Upload a GIF
agentgif upload demo.gif --title "My Demo" --command "npm test"

# Search public GIFs
agentgif search "docker build"

# Generate a terminal-themed badge
agentgif badge url -p pypi -k colorfyi --theme dracula
```

## Commands

### Authentication

```bash
agentgif login          # Open browser to authenticate
agentgif logout         # Remove stored credentials
agentgif whoami         # Show current user info
```

### GIF Management

```bash
agentgif upload <gif>   # Upload a GIF
  -t, --title <title>       GIF title
  -d, --description <desc>  Description
  -c, --command <cmd>        Command demonstrated
  --tags <tags>              Comma-separated tags
  --cast <path>              Asciicast file
  --theme <theme>            Terminal theme
  --unlisted                 Upload as unlisted
  --no-repo                  Don't auto-detect repo

agentgif search <query>   # Search public GIFs
agentgif list             # List your GIFs
  --repo <repo>              Filter by repo slug

agentgif info <gifId>     # Show GIF details (JSON)
agentgif embed <gifId>    # Show embed codes
  -f, --format <fmt>         md, html, iframe, script, all

agentgif update <gifId>   # Update GIF metadata
  -t, --title <title>       New title
  -d, --description <desc>  New description
  -c, --command <cmd>        New command
  --tags <tags>              New tags

agentgif delete <gifId>   # Delete a GIF
  -y, --yes                  Skip confirmation
```

### Badge Service

Terminal-themed package badges — a developer-native alternative to shields.io.

```bash
agentgif badge url        # Generate badge URL + embed codes
  -p, --provider <p>        pypi, npm, crates, github
  -k, --package <pkg>       Package name
  -m, --metric <m>          version, downloads, stars (default: version)
  --theme <theme>            Terminal theme (e.g. dracula)
  --style <style>            Badge style (default, flat)
  -f, --format <fmt>         url, md, html, img, all

agentgif badge themes     # List available terminal themes
```

## Configuration

Credentials are stored at `~/.config/agentgif/config.json`.

## License

MIT
