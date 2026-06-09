# Agent Runtime Kernel Design

Date: 2026-06-06
Workspace: `/path/to/triplan-agent`
Status: approved for specification

## Summary

Build a Rust-first, CLI-first, local-first agent runtime kernel for company-owned domain agents. The runtime is not another IDE coding assistant and not a desktop app. It is a minimal but durable agent engine that project teams can configure into their own specialized agents.

The v0.1 product is a single binary CLI with SQLite-backed event sourcing, checkpoint recovery, configurable agent profiles, internal multi-agent communication, cautious built-in tools, `~/.triplan-agent/.agents/skills` support, MCP support, soft sandboxing, and configurable context compaction.

The runtime is intended to replace LangGraph-style workflow infrastructure for internal agent applications. It keeps deterministic lifecycle middleware where useful, but moves dynamic context injection to asynchronous reactors that observe the run and produce targeted context patches.

## Goals

- Provide a local, debuggable, durable agent runtime that works primarily through CLI.
- Support workspace-scoped configuration, state, skills, MCP, prompts, and permissions.
- Support multiple agent profiles, each with independent prompts, tools, skills, MCP, model, and policy.
- Support multiple conversations and concurrent agent runs inside one workspace.
- Support internal agent-to-agent communication through a durable Agent Bus.
- Support context reactors for memory, code context, skill discovery, file deltas, tool summaries, and agent messages.
- Support manual, automatic, reactive, and micro compaction with configurable templates.
- Use SQLite as the event log, checkpoint store, task queue, lock table, and audit substrate.
- Support OpenAI-compatible and DashScope-compatible LLM protocols in v0.1.
- Keep built-in tools intentionally small and auditable.

## Non-Goals For v0.1

- No Electron or desktop UI.
- No SDK embedding into other applications.
- No Web UI.
- No Google A2A implementation.
- No HTTP MCP transport; v0.1 supports stdio MCP first.
- No cloud execution platform.
- No Docker, VM, or hard sandbox.
- No visual DAG builder.
- No broad built-in tool marketplace.
- No attempt to beat Cursor, Codex, Claude Code, or opencode as a polished developer product in v0.1.

## Product Positioning

The runtime should be understood as a company-owned agent operating substrate. CLI is the first control surface, not the product boundary. Future desktop, web, SDK, or protocol adapters should connect to the runtime instead of redefining it.

Compared with top coding agents, the runtime's advantage is not "better at coding on day one." Its advantage is local ownership, auditability, recoverability, multi-agent orchestration, configurable domain behavior, and event-driven context management.

The product principle is:

> CLI is the runtime's debugger and operator console. SQLite is the source of truth. Agent loops, tools, context reactors, compaction, and multi-agent communication are schedulable runtime workers.

## Core Architecture

The architecture is an event-driven runtime:

```text
CLI Console
  -> SQLite Event Log / Checkpoint
  -> Scheduler / Run Lock
  -> Agent Loop Worker
  -> LLM Provider: OpenAI or DashScope
  -> Tool Runtime
  -> Shadowbox
  -> Context Reactor Workers
  -> ContextPatch Queue
  -> Context Manager / Compaction
  -> Agent Bus
```

The CLI submits work by appending events such as `run_requested` and `user_message`. A scheduler leases tasks from SQLite. The agent loop reconstructs a prompt projection from checkpoint plus newer events, calls the model, executes tools through the tool runtime, and records semantic events.

Context reactors watch the event log and produce `ContextPatch` records. The agent loop consumes patches only at safe boundaries, such as before the next model request, after a tool round, after compaction, or after resume.

Compaction is event-driven as well. It creates compact boundaries and summary projections without deleting the full event history.

## Data Model

### Workspace

A `Workspace` is a project folder and runtime boundary. User-owned agent configuration lives under `~/.triplan-agent/.agents`, while system-owned runtime state lives under APP_DATA:

- `~/.triplan-agent/.agents/config.toml`
- `~/.triplan-agent/.agents/agents/*.toml`
- `~/.triplan-agent/.agents/skills/`
- `~/.triplan-agent/.agents/mcp.toml`
- `~/.triplan-agent/.agents/prompts/compact/*.md`
- `~/.triplan-agent/.agents/shadowbox.toml`
- `~/.triplan-agent/conversations/<workspace>/<conversation>.md`
- `APP_DATA/triplan-agent/checkpoints.sqlite3`
- `APP_DATA/triplan-agent/resources/`
- default provider and model settings
- workspace-level permissions

### AgentProfile

An `AgentProfile` describes an agent type, not a single execution. It includes:

- name and description
- system prompt and developer prompt
- provider and model
- allowed built-in tools
- allowed skills
- configured MCP servers
- shadowbox policy overrides
- context reactor policy
- compaction profile
- concurrency limits

Example profiles include `lead`, `coder`, `reviewer`, `domain_researcher`, and `ops`.

### Conversation

A `Conversation` is a multi-agent collaboration context inside a workspace. It stores the user task, participating agents, primary agent, status, compact summary chain, and conversation-level context. A conversation can contain many runs.

### Run

A `Run` is one execution instance of an `AgentProfile` inside a conversation. It stores:

- run id
- workspace id
- conversation id
- agent profile id
- parent run id
- status
- current checkpoint id
- token usage
- pause, cancel, and resume state
- lock owner and lease

### Event

Events are the fact source. Important event types include:

- `user_message`
- `run_requested`
- `run_started`
- `model_request_started`
- `assistant_message`
- `tool_call_requested`
- `tool_call_completed`
- `agent_message_sent`
- `context_patch_created`
- `compact_requested`
- `compact_started`
- `compact_completed`
- `compact_failed`
- `run_paused`
- `run_resumed`
- `run_cancelled`
- `run_completed`
- `run_failed`

Events include `workspace_id`, `conversation_id`, `run_id`, `agent_id`, `event_type`, `payload_json`, `parent_event_id`, `causation_id`, `correlation_id`, `created_at`, and a monotonic `sequence`.

Stream tokens are not stored as individual events. Only semantic events, final text, summaries, and references are persisted.

### Checkpoint

A checkpoint is a cached projection, not a fact source. It accelerates recovery:

```text
latest checkpoint + events after checkpoint -> recovered run state
```

Checkpoint content includes message projection, compact summary chain, pending tool calls, consumed patches, pending agent messages, token budget, read-file state, shadowbox state, and provider config snapshot.

### ContextPatch

A `ContextPatch` is context produced by reactors. It includes:

- target conversation, run, and agent
- kind
- priority
- TTL
- token budget
- dedupe key
- source event id
- status: `pending`, `consumed`, `expired`, or `dropped`

Kinds include agent message, memory, code context, skill discovery, file delta, tool summary, and post-compact restore.

### ToolInvocation

Tool invocations are audit records. They store tool name, input summary, output summary, artifact references, exit status, duration, policy decisions, file mutations, and related event ids.

## Multi-Agent Collaboration

v0.1 implements internal agent communication, not Google A2A.

Agents communicate through a durable Agent Bus backed by SQLite events. Agent communication never interrupts an in-flight model request. Messages and control events are consumed at safe boundaries.

Supported Agent Bus actions:

- send message
- request status
- report progress
- handoff
- pause request
- resume request
- cancel request
- set priority

Typical collaboration modes:

- `lead_agent + workers`: lead decomposes tasks, starts worker runs, receives summaries, and synthesizes results.
- `pair agents`: coder works while reviewer observes events and sends corrections.
- `human console steering`: user sends instructions to a running agent through CLI.
- `swarm-lite`: multiple workers explore in parallel under lead-agent coordination.

Worker agents cannot spawn unlimited workers. Recursion depth and concurrent run count are controlled by workspace and agent profile policy.

## Context Reactor

Lifecycle middleware remains available for deterministic gates, but it is not the main dynamic context mechanism.

Lifecycle middleware handles:

- before run
- before model
- after model
- before tool
- after tool
- after run
- permission and audit gates

Context reactors handle asynchronous context production:

- `agent_message_reactor`: converts Agent Bus messages into consumable context.
- `memory_reactor`: recalls durable or session memory.
- `code_context_reactor`: recalls relevant code snippets.
- `skill_discovery_reactor`: discovers skills based on task state.
- `file_delta_reactor`: reports changes to files the agent previously read.
- `tool_result_summary_reactor`: summarizes large tool outputs.

Reactors do not mutate prompts directly. They write `ContextPatch` rows. The agent loop selects patches under priority, TTL, dedupe, and token-budget rules.

## Context Manager And Compaction

Compaction is a runtime module, not a single prompt.

v0.1 supports:

- `manual_compact`: CLI-triggered, with optional custom instructions.
- `auto_compact`: triggered near context threshold.
- `reactive_compact`: triggered after prompt-too-long failure.
- `micro_compact`: removes or summarizes old tool results without full conversation summarization.
- `post_compact_restore`: restores necessary working state after compaction.

The default compaction template follows the Claude Code style: preserve user intent, technical concepts, files and code sections, errors and fixes, solved problems, all user messages, pending tasks, current work, and next step. The template is configurable under `~/.triplan-agent/.agents/prompts/compact/`.

Compaction emits events such as `compact_requested`, `compact_started`, `compact_completed`, `compact_failed`, and `compact_boundary_created`.

Compaction preserves the full event log. It changes prompt projection, not historical truth.

## Built-In Tools

v0.1 includes only:

- `bash`
- `file_read`
- `file_write`
- `file_edit`
- `search`
- `skill`
- `agent_message`
- `compact`
- `mcp_bridge`

Every tool call passes through `ToolRuntime`, `Shadowbox`, policy checks, output truncation, and audit recording.

### Bash

`bash` is cross-platform through a shell adapter:

- macOS/Linux: configured shell, defaulting to bash-compatible behavior.
- Windows: PowerShell by default, with explicit command adapter behavior.

It supports cwd, env allowlist, timeout, max output bytes, command policy, stdout, stderr, exit code, and artifact references.

### File Tools

File tools are workspace-bound. `file_edit` must include stale-write protection: if a file changed after the agent read it, the agent must reread before editing.

### Skill

Skills use `~/.triplan-agent/.agents/skills/<name>/SKILL.md` and progressive disclosure:

- metadata is loaded first
- full instructions are loaded only when selected
- references, scripts, and assets are loaded on demand
- each agent can have its own allowed skills

### MCP Bridge

MCP is the standard extension channel. v0.1 supports stdio MCP only. HTTP MCP is a follow-up transport after the runtime proves stdio server startup, tool discovery, tool invocation, policy checks, and audit logging. MCP tools go through the same policy and audit system as built-ins.

### Agent Message

`agent_message` is the tool entrypoint into Agent Bus. Authorization is enforced by Agent Bus and Scheduler policy, not by the sending model.

### Compact

`compact` allows an agent or user to request context compaction for a run or conversation.

## Shadowbox

Shadowbox is a soft sandbox. Its goal is control and auditability, not strong malicious-code isolation.

It enforces:

- workspace path boundary
- allowed and denied paths
- env allowlist
- command timeout
- output truncation
- destructive command policy
- network policy markers
- file mutation audit
- per-agent tool allowlist
- per-tool approval mode

It does not claim to prevent native binary escape, shell side channels, workspace malware, or host-level compromise.

## SQLite Runtime Strategy

SQLite is the control plane. It stores:

- event log
- checkpoints
- tasks
- leases
- context patches
- tool invocation audit
- agent messages
- skill registry
- MCP registry
- artifacts

Recommended tables:

- `workspaces`
- `agent_profiles`
- `conversations`
- `runs`
- `events`
- `checkpoints`
- `tasks`
- `context_patches`
- `tool_invocations`
- `agent_messages`
- `mcp_servers`
- `skill_registry`
- `artifacts`

Concurrency rules:

- enable WAL mode
- do not hold write locks during long tool or model calls
- only one agent-loop worker may own a run lease at a time
- multiple runs may execute concurrently inside a workspace
- file writes require shadowbox conflict checks
- large tool output goes to artifact files, not event JSON

Task types:

- `agent_loop_step`
- `context_reactor_step`
- `tool_invocation`
- `compact_run`
- `mcp_call`
- `checkpoint_build`

## CLI

The CLI is the v0.1 product surface.

Core commands:

```bash
triplan-agent init
agent chat
agent run
agent resume
triplan-agent status
agent events
agent runs
agent send
agent pause
agent resume-run
agent cancel
agent compact
agent tools
agent skills
agent mcp
agent config
triplan-agent doctor
```

`triplan-agent init` creates:

```text
APP_DATA/triplan-agent/
  checkpoints.sqlite3
  resources/
    prompts/
      compact/
        default.md

~/.triplan-agent/
  .agents/
    config.toml
    providers.toml
    agents/
      default.toml
      lead.toml
    skills/
    prompts/
      compact/
        default.md
    mcp.toml
    shadowbox.toml
  conversations/
    <workspace>/
      <conversation>.md
```

Conversation Markdown is user-owned history. Recent files are selected by modification time and can be injected as context before a run, similar in spirit to local coding-agent conversation recall.

Debug commands should support prompt inspection, event tailing, checkpoint inspection, context patch inspection, tool audit inspection, and JSON output.

`triplan-agent doctor` checks provider credentials, provider protocol compatibility, SQLite writability, MCP startup, skill metadata, shadowbox policy, and shell adapter availability.

## Provider Support

v0.1 supports:

- OpenAI-compatible protocol
- DashScope-compatible protocol

Providers are configured per workspace and may be overridden per agent profile. Tests should use mock providers first and real-provider smoke tests when credentials are available.

## Performance Requirements

The runtime should avoid control-plane overhead that materially affects agent loop performance.

Requirements:

- do not persist every stream token
- persist semantic events only
- avoid long SQLite write locks
- use artifacts for large outputs
- run context reactors asynchronously
- compact only at safe boundaries
- support at least four concurrent agent runs in one workspace without lock contention cascades

## Safety Requirements

v0.1 must include:

- workspace path boundary
- tool allowlist
- command timeout
- output truncation
- env allowlist
- destructive command policy
- stale file edit guard
- MCP tool approval policy
- per-agent permission profile
- full tool invocation audit

The runtime must clearly document that Shadowbox is soft isolation.

## Testing Strategy

Unit tests:

- event append and ordering
- checkpoint fold and replay
- context patch selection
- compaction prompt rendering
- tool policy decision
- path boundary checking

Integration tests:

- OpenAI-compatible mock provider
- DashScope-compatible mock provider
- bash, file, and search tools
- skill discovery and loading
- stdio MCP mock server
- SQLite task lease and retry

Agent loop tests:

- model to tool call to tool result to continuation
- prompt-too-long to reactive compact
- manual compact to resume
- tool failure to recovery
- stale edit to reread requirement

Multi-agent tests:

- lead starts worker
- human sends message to running worker
- worker consumes message at next safe boundary
- reviewer sends correction
- pause, cancel, and resume

CLI smoke tests:

- `triplan-agent init`
- `agent run`
- `agent chat`
- `triplan-agent status`
- `agent events`
- `triplan-agent compact`
- `triplan-agent doctor`

## v0.1 Acceptance Criteria

v0.1 is acceptable when it can prove:

- A workspace can initialize user data under `~/.triplan-agent/.agents` and SQLite runtime state under APP_DATA.
- User conversation history can be written as Markdown under `~/.triplan-agent/conversations` and read back for recent-context injection.
- At least two agent profiles can be configured.
- CLI can start a conversation and a run.
- Agent can call bash, file, search, skill, compact, and stdio MCP bridge.
- Every tool call has an audit record.
- CLI can send a message to a running agent.
- The target agent consumes the message at the next safe boundary.
- At least one context reactor produces and injects a `ContextPatch`.
- Manual or automatic compaction allows the agent to continue the original task.
- The process can resume after interruption from checkpoint plus events.
- Multiple agents can run concurrently in one workspace without corrupting state.
- OpenAI and DashScope protocols can run through mocks or configured real providers.
- No Electron, desktop UI, or SDK is required.

## Implementation Planning Notes

The first implementation plan should prioritize:

1. SQLite schema and event/checkpoint primitives.
2. CLI init/status/events scaffolding.
3. Provider abstraction with mock provider.
4. Single-agent loop with bash/file/search.
5. Tool audit and Shadowbox basics.
6. Manual checkpoint and resume.
7. Agent profiles and skills metadata.
8. ContextPatch minimal reactor.
9. Agent Bus message delivery.
10. Manual compact with configurable template.
11. Stdio MCP bridge.
12. Multi-agent scheduling and concurrency tests.
