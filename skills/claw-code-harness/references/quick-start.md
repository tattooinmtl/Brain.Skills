# Claw Code Quick Start Reference

## Prerequisites

- Rust toolchain with `cargo`
- `ANTHROPIC_API_KEY` or OAuth authentication

## 5-Minute Setup

```bash
# 1. Clone and build
git clone https://github.com/ultraworkers/claw-code.git
cd claw-code/rust
cargo build --workspace

# 2. Authenticate
export ANTHROPIC_API_KEY="sk-ant-..."

# 3. Health check
./target/debug/claw doctor

# 4. Start coding
./target/debug/claw
```

## First Commands to Try

### Inside REPL
```
/doctor          # Verify setup
/help            # See all commands
/status          # Current state
/model sonnet   # Switch model
```

### Outside REPL
```bash
claw prompt "explain this codebase"
claw "add error handling to main.rs"
claw --output-format json status
```

## Common Workflows

### Code Review
```bash
claw --model opus --permission-mode read-only "review this code for bugs"
```

### Autonomous Refactoring
```bash
claw --permission-mode danger-full-access "refactor to use async/await"
```

### Parallel Development
```bash
# In REPL
/subagent architect "design module X"
/subagent executor "implement module X"
/subagent reviewer "validate module X"
```

## Next Steps

- Read PHIOSOPHY.md for design principles
- Explore ROADMAP.md for future features
- Join UltraWorkers Discord for community
