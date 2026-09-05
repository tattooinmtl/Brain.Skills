# Bindings — sol2, LuaBridge, raw C API

## Which layer

| Layer | Use when | Avoid when |
|---|---|---|
| **sol2** | C++17+ host, fast iteration, default for new engines | Extreme compile times on huge usertype graphs; LuaJIT-only FFI style |
| **LuaBridge / LuaBridge3** | C++11, smaller compile surface, existing LB codebase | Need Lua 5.4 extras or rich templates sol2 already solves |
| **Raw Lua C API** | Embedding bootstrap, custom allocators, sandbox loaders, hottest marshal paths, LuaJIT specifics | Binding 40 gameplay types by hand |

Mixing is normal: raw C API for the host (`lua_newstate`, loader, allocator), sol2 `sol::state_view` on that state for gameplay types.

## Non-negotiable binding rules

1. Register types **once** at boot. Do not rebuild usertypes every reload.
2. Bind **handles and value structs**, not owning raw pointers.
3. Every bound method that takes an engine object **looks up the handle** and fails safe.
4. Do not expose STL containers as live views unless you have proven they cannot reallocate while Lua holds them. Copy small snapshots.
5. Do not let C++ exceptions propagate through Lua unless the host is built for it. Use `sol::protected_function` / `pcall` at script entry.
6. Cache bound functions used every frame (`sol::protected_function` stored on the component after first resolve).
7. Keep the stack balanced in raw API code. One `lua_gettop` assert in debug after each native function.

## sol2 patterns (default)

```cpp
sol::state_view lua(L);

lua.new_usertype<EntityId>("EntityId",
    sol::constructors<EntityId()>(),
    "index", &EntityId::index,
    "generation", &EntityId::generation);

lua.new_usertype<EntityProxy>("Entity",
    "valid", &EntityProxy::valid,
    "hp", &EntityProxy::hp,
    "set_hp", &EntityProxy::set_hp,
    "position", &EntityProxy::position,
    "set_position", &EntityProxy::set_position,
    "play_cue", &EntityProxy::play_cue,
    "queue_destroy", &EntityProxy::queue_destroy);

lua["world"] = WorldApi{&world};
lua.new_usertype<WorldApi>("World",
    "spawn", &WorldApi::spawn,
    "find", &WorldApi::find,
    "raycast", &WorldApi::raycast);
```

`EntityProxy` is a **copyable handle wrapper**, not `Entity&`. Methods resolve through `World*`.

```cpp
struct EntityProxy {
    World* world = nullptr;
    EntityId id{};
    bool valid() const { return world && world->alive(id); }
    int hp() const {
        if (auto* e = resolve()) return e->hp;
        return 0;
    }
    Entity* resolve() const { return world ? world->try_get(id) : nullptr; }
};
```

### sol2 pitfalls

- `sol::object` / `sol::function` stored on C++ components **pin** Lua objects and leak if you forget to clear them on detach.
- Binding `std::function` or lambdas that capture `sol::state` by reference dangles after reload.
- Overload sets with `const` / `T*` / `T&` produce surprising matches. Bind one obvious signature.
- `sol::as_table` on big vectors allocates and copies — fine for tools, not for 10k particles.
- `sol::readonly` for fields Lua must not mutate (IDs, generation).
- Do not use `lua.set_function` with raw owning pointers.

### Calling Lua from C++

```cpp
sol::protected_function fn = instance["on_damaged"];
if (fn.valid()) {
    sol::protected_function_result r = fn(instance, amount, source_proxy);
    if (!r.valid()) {
        sol::error err = r;
        log_script_error(err.what());
    }
}
```

Never `fn()` without protection on user content.

## LuaBridge sketch

```cpp
luabridge::getGlobalNamespace(L)
    .beginClass<EntityProxy>("Entity")
        .addFunction("valid", &EntityProxy::valid)
        .addFunction("hp", &EntityProxy::hp)
        .addFunction("set_hp", &EntityProxy::set_hp)
    .endClass()
    .beginNamespace("world")
        .addFunction("spawn", &world_spawn)
    .endNamespace();
```

Same handle rule. LuaBridge will happily bind `Entity*` — refuse that in review.

## Raw C API — host and hot marshal

Bootstrap:

```cpp
void* alloc(void* ud, void* ptr, size_t osize, size_t nsize) {
    auto* a = static_cast<ScriptAllocator*>(ud);
    if (nsize == 0) { a->free(ptr, osize); return nullptr; }
    return a->realloc(ptr, osize, nsize);
}

lua_State* L = lua_newstate(alloc, &script_alloc);
lua_atpanic(L, panic_handler);
```

Native function template:

```cpp
static int l_raycast(lua_State* L) {
    World* w = static_cast<World*>(lua_touserdata(L, lua_upvalueindex(1)));
    Vec3 from = check_vec3(L, 1);
    Vec3 to   = check_vec3(L, 2);
    Hit hit{};
    if (!w->raycast(from, to, hit)) { lua_pushnil(L); return 1; }
    push_hit(L, hit);   // table or userdata value
    return 1;
}
```

Use **full userdata + metatable** for proxies; use **light userdata** only for process-lifetime singletons (allocator, world) stored as upvalues, never as values scripts keep.

## Type design for Lua

Good to bind as values (copy):

- `Vec2/3/4`, `Quat`, `Color`, `EntityId`, small hits, enums

Good to bind as proxy userdata:

- Entity, component facade, UI widget, ability instance

Never bind:

- `std::unique_ptr`, `std::shared_ptr` to GPU objects, raw `btRigidBody*`, `ID3D11Buffer*`
- The job system
- The Lua state itself

## Review checklist

When asked to review bindings, answer these in order:

1. Can Lua hold a reference after C++ destroy? What happens?
2. Can this run off the script thread?
3. Does a script error become a C++ exception or abort?
4. Is this called in a hot loop? How many allocations per call?
5. Is the API capability-narrow enough for mods?
6. Will hot-reload invalidate the registration? (It must not.)
