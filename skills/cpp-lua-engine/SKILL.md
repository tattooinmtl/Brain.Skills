---
name: cpp-lua-engine
description: Expert workflow for custom C++ game engines with embedded Lua. Use when designing C++ vs Lua architecture, generating or reviewing bindings (sol2, LuaBridge, raw Lua C API), writing gameplay Lua against a C++ host, debugging crashes and lifetime bugs at the C++-Lua boundary, fixing Lua GC hitches, or building hot-reload, sandboxing, and modding APIs. Triggers include bind this C++ component to Lua, design the scripting API, Lua GC hitch in the game loop, hot-reload scripts, userdata lifetime, sol2 usertype, script sandbox, embed Lua, and LuaBridge binding.
metadata:
  version: "1.0"
  type: domain
---

# C++ / Lua Game Engine

Specialize in **custom C++ engines with embedded Lua**. Prefer correct lifetime and a thin scripting API over clever bindings.

Assume **Lua 5.4** for new engines and **LuaJIT 2.1 / Lua 5.1** when the user needs max throughput or existing JIT tooling. Call out version differences when they change the design. Default binding layer is **sol2** (C++17) unless the user specifies LuaBridge or raw C API.

## Workflow

1. Identify the job — architecture, binding, gameplay Lua, crash/lifetime, GC/perf, hot-reload, or sandbox/mods.
2. Decide what is C++ vs Lua using [references/architecture.md](references/architecture.md). Do not put hot-loop or ownership-heavy systems in Lua.
3. Expose a **handle-based, capability-narrow** API. Generate or review bindings with [references/bindings.md](references/bindings.md).
4. For crashes, corruption, or use-after-free, follow [references/lifetime-debug.md](references/lifetime-debug.md) before adding more binding sugar.
5. For frame spikes and GC, follow [references/performance.md](references/performance.md).
6. For reload, mods, or untrusted scripts, follow [references/hot-reload-sandbox-mods.md](references/hot-reload-sandbox-mods.md).
7. Ship code that compiles conceptually — include types, ownership, and error paths. Prefer small, copy-pasteable C++ and Lua over essays.

## Hard rules

- **C++ owns memory, threads, rendering, physics worlds, audio devices, sockets, and file I/O.** Lua gets handles and requests, never raw owning pointers.
- **Every engine object that Lua can hold is a generational handle** (index + generation), not `T*`. Invalid handles no-op or error; they must not crash.
- **Lua states are not thread-safe.** One state per thread, or marshal all script calls onto a script thread. Never run the same `lua_State` from the job system concurrently.
- **Do not cross the boundary per-field per-entity per-frame.** Batch. One C++ call that processes many items beats 10k sol2 property reads.
- **Scripts may not lock the allocator, GPU, or physics world across a yield.** No Lua yield while a C++ mutex is held.
- **Mods and editor scripts run in a sandbox by default.** No `io`, `os`, `debug`, `package.loadlib`, or unrestricted `require`.
- **Hot-reload replaces code modules, not identity.** Entity IDs and C++ components stay; Lua behavior tables are swapped with an optional `on_reload`.
- **Never bind `std::unique_ptr<T>` / raw `new`/`delete` into Lua** without a usertype destructor *and* an invalidate path for remaining references.
- When reviewing bindings, list **lifetime**, **thread**, **error**, and **hot-path** defects first. Style is last.

## Output format

When writing architecture, produce:

1. System ownership table (C++ / Lua / shared)
2. Public scripting API surface (types, events, functions)
3. Handle and error model
4. Binding sketch (sol2 unless asked otherwise)
5. Risks (GC, reload, multiplayer authority)

When writing bindings, produce C++ registration code plus a Lua usage example and a "do not do" note.

When debugging, give the most likely cause, how to confirm, then the fix. Do not shotgun-suggest `shared_ptr` for everything.

## Defaults when the user is vague

- C++17, sol2, Lua 5.4, one main-thread Lua state
- Gameplay on the main thread; jobs stay in C++
- Errors via `pcall` / `sol::protected_function` at the script entry; no C++ exception across Lua by default
- Logging and profiling hooks in the host, not in every bound method

## Reference map

| Topic | File |
|---|---|
| What lives in C++ vs Lua | [references/architecture.md](references/architecture.md) |
| sol2, LuaBridge, raw C API | [references/bindings.md](references/bindings.md) |
| Crashes, userdata, dangling hosts | [references/lifetime-debug.md](references/lifetime-debug.md) |
| GC, allocs, hot paths | [references/performance.md](references/performance.md) |
| Reload, sandbox, mod APIs | [references/hot-reload-sandbox-mods.md](references/hot-reload-sandbox-mods.md) |

Copy starter snippets from [assets/](assets/) when scaffolding a host (`entity_proxy.hpp`, `script_module.lua`) rather than inventing a new embedding each time.
