# Live E2E And Stress Test Report

**Date:** 2026-06-07 12:04:13 CST
**Branch:** `codex/agent-runtime-kernel-v0-1`
**Scope:** Real provider E2E, real MCP stdio child-process lifecycle, scheduler worker concurrency stress.

## Executive Summary

This pass implements and verifies the previously missing live/provider and runtime lifecycle coverage.

Results:

- DashScope-compatible live API E2E passed against `https://bailian.bangdao-tech.com/compatible-mode/v1` using `deepseek-v4-flash`.
- OpenAI live API E2E was executed against `https://api.openai.com/v1`, but the available local OpenAI API keys were rejected by OpenAI with `401 invalid_api_key`.
- Real MCP stdio child-process lifecycle E2E passed.
- Scheduler worker concurrency stress passed with 300 queued tasks and 12 concurrent workers.
- Default workspace initialization now uses `default_provider = "dashscope"` and `model = "deepseek-v4-flash"`.

No API keys are stored in the repository or in this report.

## Code And Test Changes

Implemented:

- `OpenAiCompatibleProvider` in `src/provider.rs`.
- `StdioMcpClient` in `src/mcp.rs`.
- `SchedulerWorker` in `src/runtime.rs`.
- Atomic task leasing plus completion/count APIs in `src/db.rs`.
- DashScope/deepseek defaults in `src/config.rs`.

Added tests:

- `tests/live_provider_e2e.rs`
- `tests/mcp_lifecycle_e2e.rs`
- `tests/scheduler_stress.rs`

Updated tests:

- `tests/cli_init.rs` now verifies DashScope/deepseek defaults.

## Verification Matrix

| Area | Command | Result |
| --- | --- | --- |
| Full Rust test suite | `cargo test` | PASS |
| Static quality gate | `cargo clippy --all-targets -- -D warnings` | PASS |
| DashScope/deepseek live E2E | `cargo test --test live_provider_e2e dashscope_live_chat_completion_returns_text -- --ignored --nocapture` with process-only `DASHSCOPE_API_KEY` | PASS |
| OpenAI live E2E | `cargo test --test live_provider_e2e openai_live_chat_completion_returns_text -- --ignored --nocapture` | FAILED: available key rejected by OpenAI |
| MCP stdio lifecycle | Included in `cargo test`; also runnable as `cargo test --test mcp_lifecycle_e2e -- --nocapture` | PASS |
| Scheduler concurrency stress | Included in `cargo test`; also runnable as `cargo test --test scheduler_stress -- --nocapture` | PASS |
| CLI default provider/model smoke | `triplan-agent init`, `triplan-agent status`, grep generated config/profile files | PASS |

## Full Test Suite Evidence

Fresh command:

```bash
cargo test
```

Observed result:

```text
tests/agent_loop.rs: 2 passed
tests/cli_init.rs: 5 passed
tests/e2e_foundation.rs: 1 passed
tests/event_store.rs: 3 passed
tests/live_provider_e2e.rs: 2 ignored
tests/mcp_lifecycle_e2e.rs: 1 passed
tests/multi_agent.rs: 1 passed
tests/scheduler_stress.rs: 1 passed
tests/tools.rs: 7 passed
Doc-tests triplan_agent: 0 failed
```

Non-ignored integration tests: 22 passed, 0 failed.

Fresh command:

```bash
cargo clippy --all-targets -- -D warnings
```

Observed result:

```text
Finished `dev` profile [unoptimized + debuginfo]
```

Exit code: 0.

## DashScope-Compatible Live API E2E

Configuration:

- Base URL: `https://bailian.bangdao-tech.com/compatible-mode/v1`
- Model: `deepseek-v4-flash`
- API key source: process environment only

Fresh command shape:

```bash
DASHSCOPE_API_KEY=<redacted> \
  cargo test --test live_provider_e2e dashscope_live_chat_completion_returns_text -- --ignored --nocapture
```

Observed result:

```text
running 1 test
test dashscope_live_chat_completion_returns_text ... ok
test result: ok. 1 passed; 0 failed
```

## OpenAI Live API E2E

Fresh command:

```bash
cargo test --test live_provider_e2e openai_live_chat_completion_returns_text -- --ignored --nocapture
```

Observed result:

```text
openai chat completion failed with status 401 Unauthorized
code: invalid_api_key
```

Interpretation:

- The OpenAI-compatible provider code path executed against the real OpenAI API endpoint.
- The available local OpenAI API keys were rejected by OpenAI.
- Error output is redacted by the provider before surfacing in test logs.
- This remains blocked on a valid OpenAI platform API key.

## MCP Stdio Child Process Lifecycle E2E

Test:

```text
tests/mcp_lifecycle_e2e.rs
```

Coverage:

- Creates a real Python stdio JSON-RPC echo server.
- Spawns it as a child process.
- Sends one JSON-RPC request over stdin.
- Reads one JSON-RPC response from stdout.
- Shuts down the child process.

Observed via `cargo test`:

```text
test stdio_mcp_client_round_trips_json_rpc_and_stops_child ... ok
```

## Scheduler Worker Concurrency Stress

Test:

```text
tests/scheduler_stress.rs
```

Coverage:

- Uses a real SQLite file database.
- Enqueues 300 tasks.
- Runs 12 concurrent scheduler workers.
- Leases tasks atomically.
- Marks tasks complete.
- Verifies every task is processed exactly once.
- Verifies all tasks end in `completed` status.

Observed via `cargo test`:

```text
test scheduler_workers_drain_tasks_once_under_concurrency ... ok
```

## CLI Defaults Smoke

Fresh command sequence:

```bash
tmpdir="$(mktemp -d)"
cargo build --bin triplan-agent --manifest-path /path/to/triplan-agent/Cargo.toml
cd "$tmpdir"
/path/to/triplan-agent/target/debug/triplan-agent init
/path/to/triplan-agent/target/debug/triplan-agent status
grep -q 'default_provider = "dashscope"' .triplan-agent/config.toml
grep -q 'model = "deepseek-v4-flash"' .triplan-agent/agents/default.toml
```

Observed key output:

```text
initialized triplan-agent workspace
workspace: triplan-agent
default_agent: default
default_provider: dashscope
CLI_DEFAULTS_OK=1
```

## Remaining Gaps

- OpenAI live E2E needs a valid OpenAI platform API key.
- Scheduler stress currently validates 300 tasks / 12 workers. Larger soaks and multi-minute runs should be added once the scheduler worker grows beyond the foundation loop.
- MCP lifecycle currently validates stdio JSON-RPC transport against a local echo server, not a third-party MCP server with tool discovery and tool invocation semantics.
