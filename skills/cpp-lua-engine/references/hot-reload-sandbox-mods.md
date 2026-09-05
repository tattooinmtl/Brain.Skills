# Hot-reload, sandboxing, modding APIs

## Hot-reload

Reload **code modules**. Do not destroy entities, C++ components, or the `lua_State`.

### Pipeline

1. File watcher (editor / dev builds only) debounces ~100–200 ms.
2. Read source, compile with `luaL_loadbuffer` / `sol::load`. On syntax error, keep the old module and report.
3. Run the chunk in a sandbox env. On runtime error during load, keep the old module.
4. Swap `package.loaded[name]` (or the host module table).
5. For each live `ScriptComponent` using that module:
   - Build a new instance table (or reuse and re-bind functions).
   - `pcall(new.on_reload, new, old)` if present.
   - Else copy known data keys (`hp` is C++ — do not copy engine state out of Lua).
6. Drop old instance refs so GC can collect the previous closure set.

### What must survive

- Entity IDs, transform, physics, inventory in C++
- Saved Lua fields you explicitly declare (`persist = {"mood", "target_id"}`)

### What must not survive

- Closures that capture old upvalues to stale C++ proxies *if* proxies were rebuilt (prefer stateless proxies)
- Bound C functions re-registered every reload (do **not** re-register usertypes)

### Module rules for reloadable Lua

```lua
local M = {}

function M.new(entity)
  return {
    entity = entity,
    acc = 0,
    on_update = M.on_update,
    on_reload = M.on_reload,
  }
end

function M.on_update(self, dt)
  self.acc = self.acc + dt
end

function M.on_reload(self, old)
  self.acc = old.acc
end

return M
```

Ban hidden globals. All state lives on `self`. That is what makes reload tractable.

### Coroutines

Break or migrate running coroutines on reload. Document that `wait()` graphs reset unless the host serializes a checkpoint. Do not try to splice a running Lua thread into a new chunk.

## Sandbox

Default stance: **designer scripts and mods are untrusted**. First-party campaign scripts can be less strict but still should not get `os.execute`.

### Environment

Create a per-module or per-mod `_ENV` that only contains:

- Safe builtins — `pairs`, `ipairs`, `next`, `type`, `tonumber`, `tostring`, `math`, `string`, `table` (no `string.dump` if you care about bytecode exfil)
- Host API — `world`, `log`, `time`, `ui` as you designed
- A `require` that only loads from the allowed package set

Remove or replace:

- `io`, `os`, `debug`, `package.loadlib`, `dofile`, `loadfile`, `load` (or wrap `load` to reject binary and to inject the sandbox env)
- `collectgarbage` in mods (host controls GC)

Lua 5.2+ / 5.4: set `_ENV`. LuaJIT / 5.1: `setfenv` on the loaded chunk.

### Budgets

- Instruction hook (`lua_sethook` count) or wall-clock cap per `pcall`
- Allocation cap via the custom allocator
- Event recursion cap (script event → C++ → script)
- No unbounded `while true` without a yield in the wait library

On budget exceed: kill the call, disable that module for the session, log. Do not tear down the whole VM unless corrupted.

### Isolation levels

| Level | How | When |
|---|---|---|
| Shared state + sandbox `_ENV` | One `lua_State` | Single-player, trusted DLC |
| Separate state per mod | Multiple states, C++ copies events across | Untrusted workshop mods |
| Separate process | IPC | Extreme (rare in games) |

Separate states cannot share Lua tables. Cross-mod talk goes through C++ messages.

## Modding API design

Version the API explicitly: `engine.api_version = 3`.

Stability rules:

- Additive functions are OK
- Never reuse a name with a new meaning
- Handles stay numeric/generational across versions
- Provide `engine.deprecated("world.kill", "use queue_destroy")` once, remove after a title update

Give mods:

- Spawn/find/query with filters
- Events (`on_damaged`, `on_interact`, `on_chat`)
- Data hooks (register an item definition table the C++ factory reads)
- UI widgets through a narrow toolkit
- Save data in a **namespaced key bag** the host serializes (`mod.save.get/set`)

Do not give mods:

- Raw pointers, FFI, filesystem outside the mod directory
- Ability to register C usertypes
- Authority over other peers' sim in multiplayer

Content pipeline: mods ship Lua + data; the host validates schemas (required fields, numeric ranges) before the definition goes live.

## Security notes

- Binary Lua chunks from disk — off for untrusted mods, or sign them.
- `string.dump` + write = bytecode leak / load attack.
- Metatable tricks on bound userdata — freeze metatables (`lua_setmetatable` then protect) or use a debug-only inspect path.
- Time bombs in `on_update` — budgets.

## Review output

When asked to design hot-reload or mods, include:

1. What reloads vs what persists
2. Sandbox `_ENV` contents
3. Budget policy
4. API surface + versioning
5. Failure behavior (disable module, not crash host)
