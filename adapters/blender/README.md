# Blender Adapter

This directory will contain the Python scripts that Toonvara invokes inside
Blender for offline rendering (Phase 3).

Scripts planned:
- `render_scene.py` — import assets, apply transforms, render image sequence
- `export_helpers.py` — coordinate conversion (Y-up → Z-up), material mapping

See `crates/studio-render/src/blender.rs` for the Rust-side orchestration.
