# Triplan Agent

Ultra-lightweight, high-performance agent runtime kernel for local-first, durable, auditable multi-agent systems.

Triplan Agent is a Rust-first runtime foundation for company-owned domain agents. It keeps agent execution local, event-driven, and recoverable with SQLite-backed state, CLI-first operations, configurable skills and tools, MCP integration, and multi-agent coordination primitives.

## Repository Layout

```text
.
|-- Cargo.toml                 # Rust workspace manifest
|-- Cargo.lock                 # Workspace lockfile
|-- crates/
|   `-- triplan-agent/         # CLI and runtime crate
|       |-- src/
|       `-- tests/
|-- docs/                      # Design notes, plans, and reports
`-- examples/                  # Demo workspaces and sample artifacts
```

## Development

```bash
cargo test -p triplan-agent
cargo run -p triplan-agent -- init
```

`triplan-agent init` separates internal runtime state from user-owned agent configuration:

```text
APP_DATA/triplan-agent/
|-- checkpoints.sqlite3
`-- resources/
    `-- prompts/

~/.triplan-agent/
`-- .agents/
    |-- config.toml
    |-- providers.toml
    |-- mcp.toml
    |-- shadowbox.toml
    |-- agents/
    |-- prompts/
    `-- skills/
```

`APP_DATA` follows the host platform conventions, such as `~/Library/Application Support/triplan-agent` on macOS, `$XDG_DATA_HOME/triplan-agent` or `~/.local/share/triplan-agent` on Linux, and `%APPDATA%\triplan-agent` on Windows. Tests and automation can override these roots with `TRIPLAN_AGENT_APP_DATA` and `TRIPLAN_AGENT_HOME`.

## CLI Builds

The CLI binary is `triplan-agent`.

Local package for the current host:

```bash
scripts/package-cli.sh
```

Local package for a specific installed Rust target:

```bash
scripts/package-cli.sh aarch64-apple-darwin
scripts/package-cli.sh macos-universal
```

Release artifacts are built by `.github/workflows/cli-release.yml` on tag pushes like `v0.1.0` or by manual workflow dispatch.

Supported release targets:

```text
triplan-agent-<version>-linux-x64.tar.gz
triplan-agent-<version>-linux-arm64.tar.gz
triplan-agent-<version>-windows-x64.zip
triplan-agent-<version>-windows-arm64.zip
triplan-agent-<version>-macos-x64.tar.gz
triplan-agent-<version>-macos-arm64.tar.gz
triplan-agent-<version>-macos-universal.tar.gz
```
