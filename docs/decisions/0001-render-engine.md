# ADR-0001: Render / animation engine choice

**Status:** Undecided

**Date:** 2026-10-13

## Context

Toonvara needs two rendering capabilities:

1. **Offline final rendering** — producing the actual MP4 output.
   For the MVP this goes through Blender (decided).

2. **Interactive viewport / animation preview** — letting users see and edit
   scenes in real time before exporting. The README spec calls for a "3D
   viewport" with orbit camera, object selection, skeleton playback, etc.

The question is which technology to use for capability (2).

## Options

### Option A: Raw wgpu + egui (README default)

Build the viewport renderer directly on wgpu, integrated into the eframe/egui
desktop app.

**Pros:**
- Full control over the rendering pipeline.
- No large framework dependency.
- Tight integration with egui (same windowing, same event loop).
- Lighter binary size.

**Cons:**
- Must build glTF loading, skeleton evaluation, animation playback, material
  pipeline, and scene-graph traversal from scratch or by assembling many small
  crates.
- Significant effort for each feature (selection, gizmos, shadows, etc.).
- No built-in ECS; must manage scene state manually or add one.

### Option B: Bevy engine

Use Bevy as the viewport runtime. Bevy provides wgpu rendering, glTF/GLB
asset loading, `AnimationPlayer`, ECS, and a plugin ecosystem.

**Pros:**
- glTF import, skeleton animation, and a scene graph work out of the box.
- ECS makes it straightforward to track object state, selection, etc.
- Active community; many viewport-adjacent features already exist as plugins
  (gizmos, editor cameras, etc.).
- Reduces custom rendering code substantially.

**Cons:**
- Large dependency tree and longer compile times.
- Bevy's renderer is opinionated — customising toon/2.5D shading may require
  fighting or replacing parts of the render pipeline.
- Integrating Bevy's winit event loop with egui panels is non-trivial
  (bevy_egui exists but adds coupling).
- Bevy's API still changes across releases (pre-1.0); upgrades carry risk.
- Desktop app architecture becomes "a Bevy app with egui panels" rather than
  "an egui app with a wgpu viewport".

### Option C: Hybrid

Use Bevy only for the viewport render surface (as a headless render-to-texture
that is blitted into an egui panel), keeping the rest of the application as
plain egui/eframe.

**Pros:**
- Gets Bevy's asset loading and animation for free.
- Keeps the outer application shell simple.

**Cons:**
- Complexity of running Bevy headlessly and synchronising its world with the
  egui application state.
- Not a well-trodden path; few examples to follow.

## Decision

**Not yet decided.** The MVP's first vertical slice (CLI → Blender → MP4)
does not require a viewport at all. We will revisit this decision before
Phase 1 (3D Viewport and Asset Import) begins.

Factors that should inform the choice:

- How much glTF/skeleton/animation code would Option A require vs. Bevy's
  built-in support.
- Whether Bevy's toon-shading story is adequate for the 2.5D visual styles
  described in FR-010.
- Compile-time and binary-size impact of Bevy on the project.
- Stability of the Bevy version available when we start Phase 1.
