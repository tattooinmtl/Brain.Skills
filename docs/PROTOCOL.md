# Plan-and-Dispatch Protocol

This is the contract every agent must follow when the user gives a prompt.

## The loop

1. **Read the prompt** (from the conversation).
2. **Build the plan**:
   ```bash
   node C:/.skills/bin/plan-from-prompt.js --prompt "<prompt>" \
        --plan-dir C:/.skills/plans/<slug>
   ```
   The planner writes `plan.md` + `phase-NN.md` files. The agent shows
   `plan.md` to the user and waits.
3. **Dispatch one phase at a time**:
   ```bash
   node C:/.skills/bin/rotate-phase.js \
        --plan-dir C:/.skills/plans/<slug> \
        --phase <N>
   ```
   This updates `.skills-meta.json` with the active phase and refreshes
   the skill-loading header at the top of `phase-<N>.md`, then prints a
   "START A NEW CONVERSATION" message.
4. **In the new conversation** the agent loads ONLY the skills listed in
   the `<!-- skills: ... -->` block at the top of `phase-<N>.md`, reads
   the rest of that file, and executes the steps.
5. After verifying the phase, the agent updates `plan.md`:
   - ticks the boxes in that phase
   - appends a one-line summary under `## Done log`
6. **Rotate** to phase N+1 and start another new conversation.

## Why rotate?

- **Auto-compact context.** Each new conversation starts clean. The carry-
  over (skill list + phase slice + done log) is intentionally tiny.
- **Skill isolation.** A phase loads only the skills it needs; the next
  phase unloads them and loads the next set. No skill pollution.
- **Auditability.** `C:/.skills/logs/rotations.log` is a complete record of
  every rotation — phase N, skills loaded, timestamp.

## Files

| Path                                                       | Owner     |
|------------------------------------------------------------|-----------|
| `C:/.skills/agents/AGENTS.md`                              | canonical |
| `C:/.skills/agents/CLAUDE.md`                              | Claude    |
| `C:/.skills/agents/GEMINI.md`                              | Gemini    |
| `C:/.skills/agents/KIMI.md`                                | Kimi      |
| `C:/.skills/agents/PI.md`                                  | Pi        |
| `C:/.skills/agents/OMNI.md`                                | Omni      |
| `C:/.skills/plans/<slug>/.skills-meta.json`                | rotator   |
| `C:/.skills/plans/<slug>/plan.md`                          | planner   |
| `C:/.skills/plans/<slug>/phase-<N>.md`                     | planner + rotator |
| `C:/.skills/plans/<slug>/report.md`                        | agent     |
| `C:/.skills/logs/rotations.log`                            | rotator   |

## Per-agent entry-point install

To install the per-agent entry file into each agent`s home dir:

```bash
node C:/.skills/bin/install-agent-md.js --harness all
node C:/.skills/bin/install-agent-md.js --harness omni
node C:/.skills/bin/install-agent-md.js --harness claude --dry-run
```
