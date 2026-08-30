#!/usr/bin/env bash
set -e
# Toolchain (rust/mold/bun/just/wasm-pack/flamegraph) is pre-installed in Dockerfile
# for cache + Zed (postCreateCommand is skipped in some editors).
# This hook only handles workspace-mounted deps that cannot be baked into the image
# (bind mount at /workspaces/artefact hides image's frontend/node_modules).

# frontend deps (must run on mounted workspace)
cd frontend && bun i

# Re-build wasm if needed (cheap no-op if up to date). Uncomment if you want it on create:
# just build wasm
