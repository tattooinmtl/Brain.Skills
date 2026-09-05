#!/bin/bash
# ----------------------------------------------------------------------------
# V4 Module: Hierarchical Swarm Topologies & Workspaces
# ----------------------------------------------------------------------------
# Creates isolated workspaces (swarms) for each Tier 2 supervisor.
# Each swarm gets its own directory structure, plan, and memory database.
# This prevents cross-project context contamination and supports parallel work.
# ----------------------------------------------------------------------------

cmd_spawn_swarm() {
  local swarm_id="$1"
  local master_objective="$2"

  echo -e "\033[0;32m[SWARM] Store Supervisor spawning Tier 2 Shift Supervisor workspace for swarm: ${swarm_id}\033[0m"

  local swarm_dir="swarms/active_${swarm_id}"
  mkdir -p "$swarm_dir"/{agents,memory,deliverables,state}

  # Delegate planning down the hierarchy to the Shift Supervisor layer
  local swarm_plan
  swarm_plan=$(query_native_coder "As a Shift Supervisor, decompose the macro-phase objective into 3 discrete worker tickets. Objective: ${master_objective}. Output valid JSON matching the task schema format.")

  echo "$swarm_plan" > "$swarm_dir/plan.json"

  # Isolate local vector memory database tracking for this specific swarm context
  sqlite3 "$swarm_dir/memory/memory.db" "CREATE TABLE IF NOT EXISTS project_embeddings (id INTEGER PRIMARY KEY, chunk_text TEXT, embedding BLOB);"

  echo "$swarm_dir"
}
