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

`triplan-agent init` separates internal runtime state from user-owned files. All user-editable configuration lives under `~/.triplan-agent/.agents`; user-owned conversation history lives under `~/.triplan-agent/conversations`; APP_DATA is reserved for runtime state and built-in resources.

```text
APP_DATA/triplan-agent/
|-- checkpoints.sqlite3
`-- resources/
    `-- prompts/

~/.triplan-agent/
|-- .agents/
|   |-- config.toml
|   |-- providers.toml
|   |-- mcp.toml
|   |-- shadowbox.toml
|   |-- agents/
|   |-- prompts/
|   `-- skills/
`-- conversations/
    `-- <workspace>/
        `-- <conversation>.md
```

`APP_DATA` follows the host platform conventions, such as `~/Library/Application Support/triplan-agent` on macOS, `$XDG_DATA_HOME/triplan-agent` or `~/.local/share/triplan-agent` on Linux, and `%APPDATA%\triplan-agent` on Windows. Tests and automation can override these roots with `TRIPLAN_AGENT_APP_DATA` and `TRIPLAN_AGENT_HOME`; user configuration still stays under the configured `TRIPLAN_AGENT_HOME`.

Provider credentials can be exported in the shell or stored in a local `.env` file. The CLI loads workspace `.env` files as well as `~/.triplan-agent/.env` and `~/.triplan-agent/.agents/.env`; keep `providers.toml` pointing at environment variable names such as `DASHSCOPE_API_KEY`, not raw key values.

Conversation Markdown can be recorded and recalled for context injection:

```bash
triplan-agent history add --workspace triplan-agent --conversation default "Important user preference"
triplan-agent history recent --limit 3 --max-bytes 12000
```

`run` supports human-readable streaming and machine-readable JSONL streaming:

```bash
triplan-agent --provider openai --model gpt-4o-mini -p "Summarize this repo"
triplan-agent run --provider openai --model gpt-4o-mini --stream "Summarize this repo"
triplan-agent run --provider openai --output-format stream-json "Summarize this repo"
```

`--output-format json` emits one final JSON object. `--output-format stream-json` emits newline-delimited events such as `run_started`, `content_delta`, `history_appended`, and `run_completed`.

## CLI Builds

The CLI binaries are `triplan` and `triplan-agent`. `triplan` is the short, user-facing command; `triplan-agent` remains available as the explicit package binary.

Start in the current workspace with lazy initialization:

```bash
triplan .
```

If user config is missing, the command creates the default files under `~/.triplan-agent`. If the selected provider has no API key, it prints the expected environment variable and `.env` setup hint.

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
