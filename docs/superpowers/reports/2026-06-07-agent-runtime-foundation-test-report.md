# Agent Runtime Foundation Test Report

**Date:** 2026-06-07 11:49:15 CST
**Branch:** `codex/agent-runtime-kernel-v0-1`
**Baseline commit before report:** `b267bcf`
**Workspace:** `/path/to/triplan-agent`

## Executive Summary

The initial verification was not a full end-to-end test. It only covered the Rust test suite and a basic CLI smoke run.

This report documents the expanded verification pass. The expanded pass adds a persistent foundation E2E test and reruns quality gates against the current worktree.

## Verification Matrix

| Area | Command | Result |
| --- | --- | --- |
| Rust test suite | `cargo test` | PASS |
| Static quality gate | `cargo clippy --all-targets -- -D warnings` | PASS |
| CLI binary build | `cargo build --bin triplan-agent --manifest-path /path/to/triplan-agent/Cargo.toml` | PASS |
| Real temporary workspace CLI smoke | `triplan-agent --version`, `triplan-agent doctor`, `triplan-agent init`, `triplan-agent status`, `triplan-agent compact ...` | PASS |
| Workspace file generation | `test -f` checks for `.triplan-agent` config/profile/prompt/MCP/shadowbox files | PASS |
| Foundation runtime E2E | `cargo test --test e2e_foundation -- --nocapture` | PASS |

## Test Suite Evidence

Fresh command:

```bash
cargo test
```

Observed result:

```text
tests/agent_loop.rs: 2 passed
tests/cli_init.rs: 4 passed
tests/e2e_foundation.rs: 1 passed
tests/event_store.rs: 3 passed
tests/multi_agent.rs: 1 passed
tests/tools.rs: 7 passed
Doc-tests triplan_agent: 0 failed
```

Total integration tests: 18 passed, 0 failed.

Fresh command:

```bash
cargo clippy --all-targets -- -D warnings
```

Observed result:

```text
Finished `dev` profile [unoptimized + debuginfo]
```

Exit code: 0.

## CLI E2E Evidence

Fresh command sequence:

```bash
tmpdir="$(mktemp -d)"
cargo build --bin triplan-agent --manifest-path /path/to/triplan-agent/Cargo.toml
cd "$tmpdir"
/path/to/triplan-agent/target/debug/triplan-agent --version
/path/to/triplan-agent/target/debug/triplan-agent doctor
/path/to/triplan-agent/target/debug/triplan-agent init
/path/to/triplan-agent/target/debug/triplan-agent status
/path/to/triplan-agent/target/debug/triplan-agent compact "Pending task: verify test report"
test -f .triplan-agent/config.toml
test -f .triplan-agent/agents/default.toml
test -f .triplan-agent/agents/lead.toml
test -f .triplan-agent/prompts/compact/default.md
test -f .triplan-agent/mcp.toml
test -f .triplan-agent/shadowbox.toml
```

Observed key output:

```text
agent 0.1.0
CLI: ok
initialized triplan-agent workspace
workspace: triplan-agent
default_agent: default
default_provider: mock
CLI_E2E_OK=1
```

## Foundation E2E Coverage

Persistent test file:

```text
tests/e2e_foundation.rs
```

The foundation E2E test covers:

- Workspace initialization and workspace config loading.
- SQLite file database creation and migration.
- Event store plus mock provider agent loop.
- Assistant message persistence in the event log.
- Checkpoint save and latest checkpoint read.
- Task enqueue and exclusive lease.
- Agent Bus message delivery into target-agent context patches.
- Shadowbox-backed file read, search, stale-safe edit, and bash tool invocation.
- `.triplan-agent/skills` metadata discovery and progressive skill body loading.
- Deterministic compaction summary sections.
- MCP JSON-RPC request shape.

## Fixes Found During Verification

The expanded quality gate found two Clippy issues:

1. `EventType::from_str` looked like it should implement `std::str::FromStr`.
2. `EventStore::append_event` has many arguments.

Resolution:

- Implemented `std::str::FromStr` for `EventType`.
- Added an explicit `#[expect(clippy::too_many_arguments)]` with a reason for the current event-log column API shape.

Commit containing the expanded E2E and quality fix:

```text
b267bcf test: add foundation e2e coverage
```

## Known Gaps

These were intentionally not covered because the corresponding runtime features do not exist in the current foundation slice:

- Real OpenAI-compatible provider E2E.
- Real DashScope-compatible provider E2E.
- Real MCP child process lifecycle E2E.
- Scheduler worker long-running loop E2E.
- Multi-agent concurrent stress test.
- Cross-platform Windows shell execution on an actual Windows host.

## Current Git State

Observed status after the expanded verification commit:

```text
## codex/agent-runtime-kernel-v0-1
?? .triplan-agent/
?? .codex/
```

Only local untracked `.triplan-agent/` and `.codex/` directories remain outside version control.
