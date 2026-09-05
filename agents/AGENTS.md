<!-- name-agnostic -->

# Agent Entry Point

This file works under ANY name (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `KIMI.md`, `PI.md`, `OMNI.md`, `<your-name>.md`). Its filename is used only as a label so the agent can identify which entry point loaded it. Behavior is identical.

## 1. Find your skills

Skills live in the skills repo. The repo is found by:

1. The `SKILLS_ROOT` env var (override), or
2. Walking up from this file until a folder containing both `bin/` and `skills/` is found, or
3. `C:/.skills` on Windows, `~/.skills` on Unix.

You can also use the helper:

```bash
node <skillsRoot>/bin/path-resolver.js   # prints resolved paths
```

## 2. Plan-and-dispatch

When the user gives you a prompt:

1. Run the planner to write `plan.md` + per-phase files into THIS folder:
   ```bash
   node <skillsRoot>/bin/plan-from-prompt.js --prompt "<prompt>" --agent "<this-file>"
   ```
2. Read `plan.md` to see the phases. Wait for the user's go-ahead.
3. For each phase, run the rotator:
   ```bash
   node <skillsRoot>/bin/rotate-phase.js --plan-dir "<this-folder>" --phase <N>
   ```
4. The rotator prints `>>> START A NEW CONVERSATION <<<`. Do exactly that.
5. In the new conversation, **load only the skills listed at the TOP of the phase-NN.md file** (the `<!-- skills: ... -->` block).

## 3. Phase file format

Every `phase-NN.md` begins with:

```
<!-- skills: <skill-1>, <skill-2>, ... -->
<!-- phase: <NN> (<Label>) -->
```

Load exactly those skills, in that order, before reading the rest of the phase file. The rest contains the goal, steps, and notes.

## 4. After each phase

1. Tick the boxes in `phase-NN.md`.
2. Append a one-line summary under `## Done log` in `plan.md`.
3. Run `rotate-phase.js --phase <N+1>`.
4. Start a new conversation.

## 5. When all phases are done

Write a final summary into `plan.md` under `## Done log` or a separate `report.md`. List: what changed, how verified, what remains.

## 6. Where skills resolve from

Given a skill name in the loading order, the actual file is:

```
<skillsRoot>/skills/<skill-name>/SKILL.md
```

No matter where this `AGENTS.md` (or any of its siblings) is copied, the skills always resolve through the path-resolver above. So you can move this file to a new repo and it just works.

## 7. Hooks

Hooks live in `<skillsRoot>/hooks/`. They fire regardless of which agent you're in. To install a hook into another agent's home dir:

```bash
node <skillsRoot>/bin/install-hook.js <hook.json> --harness <name>
```

## 8. Forbidden commands (hard block)

`rm -rf /`, `dd`, `mkfs`, `sudo rm`, `curl | sh`, `git push --force`, `gh repo delete`. See `/global-agent-guardrails`.
