---
description: Browse and invoke skills from the .skills library. Usage — /skills (list all), /skills <query> (filter), /skills use <name> (invoke).
argument-hint: [query|use <name>]
---

# /skills — browse and invoke the .skills library

The user invoked `/skills` with arguments: `$ARGUMENTS`

Your job is to help the user discover and invoke skills from the local `.skills` library.

## Where the data lives

The `.skills` repo is at: `<REPO_ROOT>`

Authoritative skills index: `<REPO_ROOT>/skills.json` — top-level skills with `name` and `path`. Each skill's `SKILL.md` YAML frontmatter has `name` and `description` for richer info.

If `skills.json` is stale or missing, refresh it first:

```bash
<REPO_ROOT>/bin/skills.exe sync
```

## How to respond based on `$ARGUMENTS`

**Empty (`/skills` alone)** — List all skills grouped by category:
- Read `<REPO_ROOT>/skills.json`
- Group by simple prefix rules: `*-coding` → Languages, `bevy-*`/`nim-*`/`threejs-*` → Game/3D, `ui-*`/`design-*` → Design, `ai-*`/`*-agent*` → AI/Agents, `*-review`, `*-debugging`, `*-plans`, `writing-*` → Engineering discipline, everything else → Other.
- One line per skill: `- <name> — <one-line hook>` (hook from SKILL.md description if easy; skip it if listing 50+).
- End with: "Reply with `/skills use <name>` to invoke, or `/skills <keyword>` to filter."

**Filter (`/skills <query>` where query is NOT `use ...`)** — Show only skills whose name or description matches the query (case-insensitive substring). Same one-line format.

**Invoke (`/skills use <name>`)** — Invoke that skill immediately via the Skill tool with `skill: "<name>"`. If not found, list close matches and ask user to pick.

## Rules

- Do NOT invoke a skill unless the user typed `/skills use <name>`. Listing is read-only.
- Do NOT read every `SKILL.md` when listing — only when the user has narrowed to <~20.
- Keep responses compact — this is a picker, not a tutorial.
- If user asks "tell me more about X", read that skill's `SKILL.md` and summarize.
