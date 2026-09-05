// Starter handle proxy. Copy into the engine and wire World::try_get.
// Lua must never see Entity*.
#pragma once
#include <cstdint>

struct EntityId {
    std::uint32_t index = 0;
    std::uint32_t generation = 0;
    explicit operator bool() const { return generation != 0; }
};

struct World;
struct Entity;

struct EntityProxy {
    World* world = nullptr;
    EntityId id{};

    bool valid() const;
    Entity* resolve() const;  // nullptr if dead; never persist this pointer in Lua
};

// sol2: register EntityProxy methods only. Do not register Entity.
