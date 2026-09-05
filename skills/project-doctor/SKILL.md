---
name: project-doctor
command: /doctor
description: Inspect a project for missing files, broken paths, invalid configs, and startup blockers.
---

# Project Doctor

## Purpose
Inspect a project folder and identify missing files, broken paths, invalid configs, failed imports, and startup blockers.

## When to use
Use this skill when the user runs:

/doctor <project-path>

## Rules
- Read the project before suggesting fixes.
- Never guess missing files.
- Check package.json, server files, frontend files, config files, and runtime paths.
- Report blockers first.
- Give exact commands for Windows PowerShell when the project is on Windows.

## Output
1. Status
2. Blocking errors
3. Suspicious issues
4. Exact fixes
5. Retest commands

## Compatibility

This skill is part of the unified kit at `C:/.skills/`. See `C:/.skills/agents/AGENTS.md` for the plan-and-dispatch protocol that rotates skills into the conversation per phase. References to legacy home paths (e.g. `~/.omni/skills/`, `~/.kimi-code/skills/`) in this skill have been rewired to `C:/.skills/skills/`.
