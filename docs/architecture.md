# Toonvara Architecture

## Overview

Toonvara is a Cargo workspace organised into library crates (under `crates/`)
and application targets (under `apps/`). Both the CLI and the desktop GUI
depend on the same core libraries so that business logic is never duplicated
in the UI layer.

```
apps/cli          – clap-based CLI (`toonvara` binary)
apps/desktop      – eframe/egui desktop application
crates/studio-core  – canonical data model (Project, Scene, Camera, …)
crates/studio-json  – JSON video-definition schema, validation, parsing
```

Additional crates (studio-assets, studio-animation, studio-scene,
studio-render, etc.) will be introduced when a development phase requires them.
See README.md section 9 for the full target structure.

## Data flow

```
JSON file  ──▶  studio-json (parse + validate)  ──▶  studio-core types
                                                        │
                CLI render command  ◀───────────────────┘
                        │
                        ▼
              Blender adapter (Phase 3)  ──▶  image sequence
                        │
                        ▼
                 FFmpeg encode  ──▶  MP4
```

## Key design rules

1. **The desktop UI never owns business logic.** The CLI and desktop app share
   the same core crates.
2. **One offline renderer (Blender) for the MVP.** A native wgpu/Bevy preview
   may follow — see `docs/decisions/0001-render-engine.md`.
3. **Frames are the canonical timeline unit.** Seconds are converted at the
   project's configured FPS.
4. **Asset paths are project-relative.** No absolute paths in JSON or project
   files.

## Known constraints

### Text overlays — Latin-only for MVP

Toonvara targets English / Latin-script global cartoons for the MVP. Subtitle
and text overlay support is Latin-only for now.

### Khmer / complex-script text shaping (DEFERRED)

Khmer script requires complex text shaping (reordering, ligatures,
above/below-base marks) via HarfBuzz or an equivalent engine (e.g. `cosmic-text`).
This is a real requirement for future localisation but is **deferred** — it is
not an MVP constraint. When subtitle or text overlay support is extended beyond
Latin script, the implementation must be validated against Khmer sample text
before it is considered complete.
