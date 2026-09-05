# Crashes and lifetime at the C++–Lua boundary

Most "Lua crashed the engine" bugs are **C++ lifetime** bugs that Lua merely triggered.

## Symptom → likely cause

| Symptom | First suspects |
|---|---|
| Crash in bound method after entity despawn / level unload | Dangling `T*` userdata; no generation check |
| Crash after script hot-reload | C++ stored `sol::function` / registry ref into old module; dangling upvalue |
| Crash / corruption on quit | `lua_close` while C++ still has `sol::reference`; or usertype dtor touches freed World |
| Random heap corruption during GC | Usertype dtor double-free; lightuserdata to moved object; stack smash in raw API |
| Hang or crash from worker thread | `lua_State` used off the script thread |
| `PANIC: unprotected error` | Error from C API without `pcall`; `lua_atpanic` path |
| Leak that grows with playtime | `sol::object` / registry refs not unref'd on detach; hidden cycle (C++ holds Lua holds C++) |
| Works until first GC, then dies | Missing `__gc` or userdata that assumed no collect |

## Handle validity is the fix

Do not "just use `shared_ptr`". Shared ownership across a scripting VM hides leaks and extends GPU/physics lifetime past the world.

Required pattern:

1. C++ object lives in a slot map / generational arena.
2. Lua userdata stores `{type_tag, index, generation}` only.
3. Every native entry point resolves or returns a safe failure.
4. On destroy, generation++. Outstanding Lua refs become inert.

On world teardown:

1. Call script `on_detach` for remaining instances (`pcall`).
2. Clear all C++-held Lua refs (`sol::function`, registry indices).
3. Destroy C++ objects.
4. `lua_close` last.

Never destroy the world *after* `lua_close` if usertype `__gc` still needs the world. Either:

- Make `__gc` a no-op after a host `shutdown` flag, or
- Drop all userdata (clear tables, full GC) before tearing down C++ systems.

## C++ holding Lua

Safe:

- Registry reference created at attach, `luaL_unref` on detach
- `sol::protected_function` member cleared in `ScriptComponent` destructor *while the state is alive*

Unsafe:

- Storing `sol::function` in a static
- Capturing `sol::table` in a `std::function` posted to a job
- Registering a C++ callback with Lua that outlives the state

If a C++ system must call Lua later, store an `EntityId` + event name, not a raw function, and re-resolve at fire time.

## Lua holding C++

Unsafe examples to reject in review:

```cpp
lua.set_function("get_entity", [&](int id) -> Entity* {
    return world.get(id); // raw pointer into a vector
});
```

```lua
local body = entity.rigid_body  -- bound as btRigidBody*
entity:queue_destroy()
body:applyImpulse(...)          -- UAF
```

Replace with proxies that resolve each call.

## Stack and API mistakes

- Forgetting `lua_pop` after `lua_getfield` in a loop
- Returning 2 values but `return 1`
- Using `lua_tostring` on a non-string that errors inside a raw `__index`
- `lua_yield` from a C function not called via `lua_resume`
- Mixing Lua 5.1 and 5.4 stack APIs (`lua_objlen` vs `lua_rawlen`, integer types)

In debug builds, wrap every `lua_CFunction` with a top-delta assert.

## Threads

`lua_State` is single-threaded. Patterns that work:

- **Main-thread only scripts** (default)
- One state per worker, no shared tables
- `lua_newthread` coroutines **on the same state**, resumed only on the script thread

Patterns that crash:

- Job system calling bound methods
- Physics contact callback (often on a physics thread) directly invoking Lua
- Audio mixer thread calling `play_finished` into Lua

Marshal events onto a lock-free queue; script thread drains it.

## How to confirm

1. Repro with `ASAN` + `lua_sethook` / instruction count logging.
2. Log resolve failures (invalid generation) — a spike means Lua is racing destroy.
3. Break on `lua_close` and inspect remaining `sol::reference` members.
4. For GC issues, force `collectgarbage("collect")` after despawn in a debug command.
5. For thread issues, poison a `thread_local` owner id in every native entry.

## Fixes that are usually wrong

- Wrapping every engine type in `shared_ptr` "so Lua can keep it"
- Disabling GC
- Catch-all `try/catch(...)` around the game loop
- Binding `debug` / `inspect` into ship builds to "see what happened"

Prefer invalidate-on-destroy, explicit detach, and a script error log with source+line.
