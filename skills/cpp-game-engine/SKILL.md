---
name: cpp-game-engine
description: >-
  Comprehensive guide to understanding and building C++ 3D game engines and their core subsystems.
  Covers game loops, component architecture, 3D math & transform hierarchies, rendering pipelines,
  physics & spatial queries, asset management, audio, scene serialization, command buses, and Lua scripting.
---

# C++ 3D Game Engine Architecture & Subsystems Guide

A comprehensive guide explaining the core functions, data structures, and subsystems of modern 3D Game Engines in C++20.

---

## 1. Subsystem Architecture Overview

A modern C++ game engine is divided into decoupled, cooperating subsystems:

```
┌──────────────────────────────────────────────────────────────┐
│                    Editor / UI Layer                         │
│         (ImGui, Viewport Framebuffer, Gizmos, Inspector)     │
├──────────────────────────────────────────────────────────────┤
│                    Gameplay & Scripting                      │
│         (Lua 5.4 Sandbox, GameplayLoop, AICommandBus)        │
├──────────────────┬───────────────────┬───────────────────────┤
│ Scene & Entities │ Physics & Spatial │ Rendering & Shaders   │
│ (Component Model,│ (PhysX/Bullet,    │ (OpenGL/Vulkan/DX12,  │
│  Transform Tree) │  Raycasts, AABB)  │  PBR, Mesh, Materials)│
├──────────────────┴───────────────────┴───────────────────────┤
│                     Core & Foundation                        │
│ (AssetDatabase GUIDs, Memory/RAII, Math/GLM, File I/O, Audio)│
└──────────────────────────────────────────────────────────────┘
```

---

## 2. Core Engine Subsystems & Their Functions

### A. The Core Game Loop & Timing (`GameplayLoop`)
- **Function**: Coordinates continuous simulation, input polling, fixed physics stepping, variable rendering updates, and frame pacing.
- **Key Concepts**:
  - `DeltaTime` ($\Delta t$): Time elapsed since the last frame in seconds. Prevents speed variations across different frame rates.
  - `FixedUpdate` (Accumulator pattern): Runs at fixed intervals (e.g. 50 Hz / 20ms) for deterministic physics and collision resolution.
  - `Update`: Runs once per rendered frame for variable logic, animations, and camera tracking.
- **C++ Pattern**:
  ```cpp
  void GameLoop::Tick() {
      auto currentTime = std::chrono::high_resolution_clock::now();
      float frameTime = std::chrono::duration<float>(currentTime - lastTime_).count();
      lastTime_ = currentTime;

      accumulator_ += frameTime;
      while (accumulator_ >= fixedDeltaTime_) {
          PhysicsSystem::Step(fixedDeltaTime_);
          accumulator_ -= fixedDeltaTime_;
      }

      ScriptingSystem::Update(frameTime);
      RenderSystem::RenderFrame();
  }
  ```

---

### B. Component Architecture (`GameObject` & `Component`)
- **Function**: Decouples entities from rigid inheritance hierarchies. Game objects are lightweight containers of modular components.
- **Standard Components**:
  - `TransformComponent`: Position, Euler rotation / Quaternion, Scale, Parent pointer.
  - `MeshFilterComponent`: Geometry handle, primitive type, vertex/index buffer reference.
  - `MeshRendererComponent`: PBR Material reference (`.gfmat`), textures, shaders, cast/receive shadows.
  - `ColliderComponent`: Box, Sphere, Capsule, Mesh collider shapes, trigger flags, physics materials.
  - `LightComponent`: Directional, Point, Spot light parameters (color, intensity, range, shadows).
  - `CameraComponent`: FOV, Near/Far clipping planes, projection matrix (Perspective / Orthographic).
  - `AudioSourceComponent`: Audio clip GUID, volume, pitch, 3D spatialization flags.
  - `ScriptComponent`: Attached script path (e.g. Lua), property overrides, state table.

---

### C. 3D Math & Transform Hierarchies
- **Function**: Calculates object positions, rotations, and scales relative to parent objects and converts them to global world-space coordinates.
- **Coordinate Systems**:
  - Right-Handed vs. Left-Handed; Y-Up (standard in OpenGL / GameForgerAI) vs. Z-Up (Blender).
- **Matrix Composition**:
  $$\text{ModelMatrix} = \text{Translate}(T) \times \text{Rotate}(R) \times \text{Scale}(S)$$
- **Hierarchy Propagation**:
  $$\text{WorldTransform}_{\text{child}} = \text{WorldTransform}_{\text{parent}} \times \text{LocalTransform}_{\text{child}}$$
- **C++ Implementation**:
  ```cpp
  glm::mat4 TransformComponent::GetLocalMatrix() const {
      glm::mat4 model = glm::translate(glm::mat4(1.0f), position);
      model = glm::rotate(model, glm::radians(rotationEuler.y), glm::vec3(0, 1, 0));
      model = glm::rotate(model, glm::radians(rotationEuler.x), glm::vec3(1, 0, 0));
      model = glm::rotate(model, glm::radians(rotationEuler.z), glm::vec3(0, 0, 1));
      return glm::scale(model, scale);
  }
  ```

---

### D. Rendering Pipeline & Shaders
- **Function**: Submits geometry, materials, and lighting to the GPU via OpenGL/Vulkan/DirectX.
- **Pipeline Stages**:
  1. **Mesh Upload**: Vertex Buffer Objects (VBO), Vertex Array Objects (VAO), Element Buffer Objects (EBO).
  2. **Uniforms & Matrices**: View Matrix ($\text{Camera}^{-1}$), Projection Matrix ($\text{Perspective}$), Model Matrix.
  3. **PBR Shading**: Physically Based Rendering using Albedo, Normal Maps, Metallic/Roughness, and Ambient Occlusion.
  4. **Framebuffer Output**: Renders scene to an off-screen texture (`fboTextureID`) which ImGui displays inside the editor viewport window.

---

### E. Physics Subsystem & Spatial Queries
- **Function**: Detects collisions, applies forces (gravity, friction, impulses), and resolves constraints.
- **Broadphase vs Narrowphase**:
  - *Broadphase*: Axis-Aligned Bounding Box (AABB) trees / spatial hashing to quickly eliminate non-colliding pairs.
  - *Narrowphase*: GJK (Gilbert-Johnson-Keerthi) or exact mesh/primitive collision intersection.
- **Raycasting**:
  - Traces a 3D ray $\vec{R}(t) = \vec{O} + t \cdot \vec{D}$ into the scene to find intersected objects for mouse picking, line-of-sight, or weapon hits.

---

### F. Asset Management (`AssetDatabase`) & GUIDs
- **Function**: Maps content assets (models, textures, sounds, scripts) to globally unique identifiers (GUIDs).
- **Principles**:
  - Scene files store GUID references (e.g. `uuid: "a7b3c2..."`), never brittle relative file paths.
  - Companion `.meta` files track asset import settings and GUID stability across folder renames.

---

### G. Scene Graph & Safe Serialization
- **Function**: Saves and loads entire world states to structured JSON or binary files (`.gfscene`).
- **Atomic Writes**:
  - Always write to a temporary file (`scene.gfscene.tmp`), flush to disk, and atomically rename to `scene.gfscene` to prevent corruption on crash.

---

### H. Command Bus & Undo/Redo System (`AICommandBus`)
- **Function**: Implements the Command Pattern for all state-mutating operations.
- **Structure**:
  - `Execute()`: Applies the mutation.
  - `Undo()`: Reverts the mutation to its previous snapshot state.
  - Maintains `undoStack` and `redoStack`.

---

### I. Scripting Engine & Sandboxing (Lua 5.4)
- **Function**: Enables fast gameplay logic iterations without recompiling C++.
- **Key Practices**:
  - Per-script `_ENV` sandboxing to prevent global pollution.
  - Execution hook (`lua_sethook`) to abort infinite loops.
  - Exposing C++ bindings: `Transform`, `Input`, `Physics`, `Audio`, and `GameObject`.

---

## 3. Verification Checklist for C++ Game Engine Code

Before committing changes to any engine subsystem:
1. **Memory & Ownership**: Is memory managed via `std::unique_ptr` / `std::shared_ptr`? No naked `new`/`delete`.
2. **Spatial Correctness**: Are matrices multiplied in the correct order ($\text{Translate} \times \text{Rotate} \times \text{Scale}$)?
3. **Undo/Redo Integrity**: Did scene mutation go through `AICommandBus`?
4. **Zero Compiler Warnings**: Verify clean compilation under MSVC / Clang with C++20.
5. **Unit & Math Tests**: Are transforms and physics behaviors backed by GoogleTest assertions?
