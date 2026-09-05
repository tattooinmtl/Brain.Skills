# Performance — GC, allocation, hot paths

## First question

Is the cost **Lua VM / GC**, **binding marshal**, or **C++ work Lua requested**? Profile before moving systems. A Tracy/`optick` zone around `ScriptHost::tick` plus Lua alloc bytes this frame is enough to start.

## Budgets

Reasonable defaults for a 16.6 ms frame (adjust per genre):

- Script tick wall time — 0.5–2.0 ms
- New Lua allocations — as close to 0 on the hot path as possible
- Bound C++ calls from Lua — hundreds, not tens of thousands
- Entities with `on_update` in Lua — dozens to low hundreds, not every actor

If designers need thousands of scripted actors, they get **C++ systems + Lua config**, not per-entity `on_update`.

## Crossing the boundary

Expensive:

```lua
for i = 1, #enemies do
  local e = enemies[i]
  local p = e:position()      -- 3 floats marshal
  e:set_position(p.x + 1, p.y, p.z)
end
```

Cheap:

```lua
world.translate_team(TEAM_ENEMY, 1, 0, 0)  -- one call, C++ iterates
```

or batch:

```lua
world.set_positions(ids, xs, ys, zs)
```

Review rule: if a Lua loop calls a bound method N times over the same system, offer a batched C++ verb.

## Allocation and GC

Lua GC hitches come from **rate of garbage**, not from "GC being on".

Hot-path sins:

- Building a new table every frame (`{x=..., y=...}` for every entity)
- String format in `on_update` (`"hp="..hp`)
- Closures created per event (`function() unit:fire() end` inside a loop)
- `sol::table` created in C++ per call
- Returning `std::vector` via sol2 as a table every query

Tactics:

- Reuse tables: `pos.x, pos.y, pos.z = e:position_xyz()` into a scratch table
- Return multiple numbers instead of a vector table when easy
- Intern event names as pre-interned strings / light userdata tokens
- Pre-create callback closures at `on_attach`
- For Lua 5.4, use generational GC (`collectgarbage("generational")`) and step during idle / camera cuts
- For LuaJIT, stay in the JIT-able subset; avoid traces that abort on bound C calls in tight loops — pull the loop into C++

Host allocator:

- Route `lua_newstate` through the engine allocator
- Track bytes in/out for a live "script heap" HUD
- Optional hard cap that errors the sandbox instead of growing forever

Do not pause the whole game on `collectgarbage("collect")` in shipping. Step it.

## LuaJIT vs Lua 5.4

| | LuaJIT 2.1 | Lua 5.4 |
|---|---|---|
| Throughput | Best for numeric gameplay loops | Slower VM; fine for sparse logic |
| Integers | Numbers are doubles | True integers |
| GC | Incremental; allocation-sensitive | Generational option |
| Bindings | FFI exists but **do not expose raw engine FFI to mods** | sol2 / C API |
| Use | High entity counts, existing LJ codebase | New engines, simpler embedding, 5.4 features |

If the user needs both, keep gameplay logic JIT-friendly *or* accept that hot work moves to C++.

## sol2 hot-path notes

- Resolve `sol::protected_function` once; store on the component.
- Avoid `lua["name"]` lookups every frame — intern in a C++ function table.
- `sol::state::script` / `script_file` is for load time, not ticks.
- `protected_function` has a cost; for trusted internal scripts a raw call can be used, never for mods.

## C++ side costs Lua triggers

Lua saying `spawn()` 500 times in one frame is a C++ hitch. Give Lua:

- pooled spawn (`spawn_n`)
- deferred queues processed with a cap
- time-sliced loads

Same for raycasts: `world.raycast_batch`.

## Profiling checklist

1. Zone `ScriptHost::tick` and per-module `pcall`.
2. Count bound calls / frame (atomic increment in debug).
3. Plot script heap bytes and GC pause time.
4. List modules by `on_update` cost; make the expensive ones event-driven.
5. Only then micro-optimize Lua locals / `local ipairs`.

## Advice format

When the user reports a "Lua GC hitch in the game loop":

1. Ask what the script heap graph does (climb + drop vs flat).
2. Hunt per-frame tables and string builds.
3. Hunt per-entity bound getters.
4. Move the inner loop to C++ or batch it.
5. Tune GC *last*.
