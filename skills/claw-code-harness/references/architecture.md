# Claw Code Architecture Reference

## System Overview

Claw Code is built as a modular Rust workspace with 9 crates:

```
rust/
├── Cargo.toml              # Workspace root
└── crates/
    ├── api/               # Provider clients
    ├── commands/          # Slash command registry
    ├── compat-harness/    # TS manifest extraction
    ├── mock-anthropic-service/  # Testing mock
    ├── plugins/           # Plugin management
    ├── runtime/           # Core runtime
    ├── rusty-claude-cli/  # CLI binary
    ├── telemetry/         # Usage tracking
    └── tools/            # Tool implementations
```

## Crate Details

### api/ - Provider Layer
- Anthropic API client
- OpenAI-compatible API client
- SSE streaming support
- Request preflight checks
- Context window management
- Auth: API key + OAuth bearer

### runtime/ - Core Engine
- ConversationRuntime orchestration
- Config loading (5-file cascade)
- Session persistence
- Permission policy enforcement
- MCP client lifecycle
- System prompt assembly
- Usage tracking

### rusty-claude-cli/ - CLI Surface
- REPL implementation (rustyline)
- One-shot prompt mode
- Direct CLI subcommands
- Streaming display
- Tool call rendering
- Argument parsing

### tools/ - Tool System
Built-in tools:
- Bash (shell execution)
- ReadFile (file reading)
- WriteFile (file creation/modification)
- EditFile (precise modifications)
- GlobSearch (path-based discovery)
- GrepSearch (pattern search)
- WebSearch (internet research)
- WebFetch (URL content)
- Agent (sub-agent spawning)
- TodoWrite (task tracking)
- NotebookEdit (Jupyter notebooks)
- Skill (skill invocation)
- ToolSearch (tool discovery)

### commands/ - Slash Commands
- Command registry
- Tab completion
- Help text generation
- JSON/text rendering

### plugins/ - Plugin System
- Plugin metadata
- Install/enable/disable flows
- Hook integration
- Plugin tool definitions

### telemetry/ - Usage Tracking
- Session trace events
- Usage metrics
- Cost tracking

## Data Flow

```
User Input
    │
    ▼
rusty-claude-cli (CLI/REPL)
    │
    ▼
runtime (ConversationRuntime)
    │
    ├──► api (Provider requests)
    │        │
    │        ▼
    │    tools (Tool execution)
    │        │
    │        ◄─┘
    │
    ├──► plugins (Extended capabilities)
    │
    ├──► telemetry (Usage tracking)
    │
    └──► session persistence
```

## Event System

Claw Code emits typed events for clawhip integration:

- `lane.started`
- `lane.ready`
- `lane.prompt_misdelivery`
- `lane.blocked`
- `lane.red`
- `lane.green`
- `lane.commit.created`
- `lane.pr.opened`
- `lane.merge.ready`
- `lane.finished`
- `lane.failed`
- `branch.stale_against_main`

## Configuration Cascade

Runtime config loaded in priority order:

1. `~/.claw.json`
2. `~/.config/claw/settings.json`
3. `/.claw.json`
4. `/.claw/settings.json`
5. `/.claw/settings.local.json`

Later entries override earlier ones.
