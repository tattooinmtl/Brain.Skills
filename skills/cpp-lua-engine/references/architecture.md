# C++ vs Lua architecture

## Ownership split

| Lives in C++ | Lives in Lua | Shared carefully |
|---|---|---|
| Window, input devices, time | Input *policy* (bindings, contexts) | Input events as small POD copied into Lua |
| Renderer, GPU resources, meshes | Material *selection*, debug draw requests | Render handles; never ID3D/Vk objects |
| Physics world, colliders, raycasts | Gameplay reactions to hits | Query results as value structs |
| Audio mixer, voices | When to play / which cue | Cue IDs |
| Asset IO, packing, cooking | Data tables that *name* assets | Asset IDs resolved in C++ |
| Job system, threads, fibers | Nothing concurrent | Script run only on the script thread |
| Net sockets, replication bitstreams | Rules that *decide* what to send (single-player / co-op) | Authoritative sim stays C++ in competitive games |
| ECS storage, archetypes, iteration | Per-entity behavior glue | Script components store a module name + instance table |
| Deterministic sim step | UI, tools, juice, AI *decisions* | AI uses C++ sensors; Lua picks among scored options |

If a system runs every frame on thousands of objects, it is C++. If a designer or modder must change it weekly, it is Lua (or data).

## Recommended host shape

```
Engine
  TaskGraph / JobSystem          // C++ only
  World / ECS                    // C++ storage
  ScriptHost                     // one lua_State (or one per isolated VM)
    TypeRegistry                 // usertypes registered once
    HandleTable                  // id + generation -> C++ object
    EventQueue                   // C++ -> Lua, drained on script tick
    ModuleLoader                 // sandbox-aware require
    Budget                       // instructions / time / alloc
  Gameplay systems               // C++ update; may invoke ScriptHost.Call
```

Tick order that avoids re-entrancy bugs:

1. Input + time
2. C++ simulation (physics, animation pose, movement)
3. Drain C++ → Lua events; run scheduled Lua (AI, abilities, UI)
4. Apply Lua *requests* (spawn, impulse, play cue) queued during step 3
5. Render / audio

Never let Lua call back into a system that is mid-update. Queue requests.

## Handles, not pointers

Lua must only see:

```cpp
struct EntityId { uint32_t index; uint32_t generation; };
```

C++ resolves `EntityId` through a slot map. Destroyed entities bump `generation`. Bound methods start with `Entity* e = world.try_get(id); if (!e) return {};`.

Same pattern for `SoundId`, `MeshId`, `ColliderId`, `ScriptRef`.

## What a scripting API should look like

Narrow, verb-oriented, hard to misuse:

```lua
local e = world.spawn("prefab.grunt")
e:set_team(2)
e:play_cue("alert")
e.on_damaged = function(self, amount, source)
  if self:hp() <= 0 then self:queue_destroy() end
end
```

Avoid:

- Binding the entire ECS (`get_component<T>` from Lua for every T)
- Setters for every transform field called 6 times per entity per frame
- Letting Lua allocate engine objects with C++ constructors and no factory

Prefer factories (`world.spawn`, `physics.raycast`) and events (`on_damaged`) over omnipotent reflection.

## Multiplayer authority

- Competitive / lockstep / server-auth: **simulation in C++**. Lua may configure tunables and presentation.
- Single-player / co-op PVE: Lua may own ability graphs and quest logic; still keep movement integration and physics in C++.
- Never let a client mod Lua dictate other players' authoritative state.

## Script component pattern

C++ component:

```text
ScriptComponent
  module: string          -- "ai/grunt"
  instance: Lua table     -- { entity=Id, ... } created on attach
  flags: wants_update, wants_events
```

Lifecycle calls from C++ (always `pcall`):

- `on_attach(self)`
- `on_update(self, dt)` — only if flagged; default off for most entities
- `on_event(self, name, ...)`
- `on_detach(self)`
- `on_reload(self, old)` — see hot-reload reference

Most entities should **not** have `on_update`. Drive them with events and C++ systems.

## Data vs code

Put numbers and lists in data (Lua tables loaded as config, or JSON/TOML cooked to binary). Put branching in Lua modules. Put inner loops in C++.

Example: bullet spread table in data; fire-mode state machine in Lua; projectile integration in C++.

## Decision checklist

Put it in C++ if any of these are true:

- Touches GPU, audio device, socket, or filesystem
- Must be deterministic across machines
- Runs more than ~1k times per frame
- Owns memory other systems iterate

Put it in Lua if:

- Designers iterate daily
- Logic is sparse (dozens of entities, not tens of thousands)
- Failure should degrade to a log, not a crash
- Mods need to replace it
