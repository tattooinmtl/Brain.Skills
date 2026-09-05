# Plan: Build a small Rust CLI that audits a JSON file for top-level type errors

Generated: <auto>
Phases: 6

## Done log

<!-- append one-line summary per phase as it completes -->

## Phases

### Phase 1: Understand

Restate the goal; ask one focused question if needed.

## Skills to load

- brainstorming
- prompt-me
- before-building

## Steps

- [ ] (auto — first run starts here)

### Phase 2: Explore

Map the stack; gather context before any change.

## Skills to load

- using-addon-skills
- codebase-inspection
- rag_search
- find_symbol
- dev_env_report

## Steps

- [ ] Inspect target repo (`project_inspect`)
- [ ] Confirm Rust toolchain present (`where_is cargo`)
- [ ] Identify any existing JSON parsing code

### Phase 3: Plan

Write phases to todos; decide what "done" means.

## Skills to load

- plan
- writing-plans
- next-decision

## Steps

- [ ] Decide CLI shape (argparse vs clap)
- [ ] Decide output format (text vs JSON)
- [ ] Write a failing test stub

### Phase 4: Implement

Apply changes in small increments, one file at a time.

## Skills to load

- coder-ai-senior-developer
- dispatching-parallel-agents
- git-worktrees

## Steps

- [ ] `cargo new --bin json-audit`
- [ ] Add `serde` + `clap` to `Cargo.toml`
- [ ] Implement the audit logic
- [ ] Wire CLI args

### Phase 5: Verify

Run tests/build/lint; paste command + output.

## Skills to load

- test-driven-development
- systematic-debugging
- verification-before-completion
- vibe-code-auditor

## Steps

- [ ] `cargo build` → exit 0
- [ ] `cargo test`  → all pass
- [ ] `cargo clippy -- -D warnings` → clean
- [ ] Manual run against a malformed fixture

### Phase 6: Report

Summarise: what changed, how verified, what remains.

## Skills to load

- requesting-code-review
- handoff
- finishing-a-development-branch

## Steps

- [ ] Write `report.md`
- [ ] Open PR if requested
- [ ] Run `git_diff` and paste final summary

## Notes

- Rotate to next phase: `node C:/.skills/bin/rotate-phase.js --plan <this-file> --phase <N>`
- This file is the source of truth — keep it updated.
