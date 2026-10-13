# Cartoon Studio

**3D Character Creator · Animation Editor · JSON Video Generator**

**Version:** 0.1.0 (Proposed)  
**Status:** Planning / Pre-development  
**Primary language:** Rust  
**Platform:** macOS, Windows, Linux  
**Architecture:** Desktop-first, offline-first, modular

---

## 1. Project Overview

Cartoon Studio is a standalone desktop application that allows users to create, customize, animate, and render 3D cartoon characters through two complementary workflows:

1. **Visual Mode** — Create characters and animation using graphical editing tools.
2. **JSON Mode** — Define scenes, characters, actions, cameras, and audio using structured JSON, then generate cartoon videos automatically.

Both modes must use the same project data model, animation system, and asset library.

The application must support creating short animations, cartoon episodes, educational content, promotional videos, and reusable character assets.

### 1.1 Vision

Make professional-looking cartoon creation accessible to users without requiring advanced 3D modeling or programming skills.

### 1.2 Primary Goals

- Provide an intuitive character-creation experience.
- Support customizable humans, animals, and fictional characters.
- Provide real-time 3D character and scene previews.
- Support skeleton-based character animation.
- Generate animated videos from JSON.
- Allow users to manually edit JSON-generated scenes.
- Support reusable models, animations, and environments.
- Export finished projects as video files.
- Provide a scalable architecture for future features.
- Remain usable offline without requiring a cloud account.

### 1.3 Target Users

- Independent animation creators
- YouTube cartoon creators
- Content creators and educators
- Small animation studios
- Game developers
- Developers automating video production
- Beginners learning character animation

### 1.4 Product Principles

- Simple user experience, powerful underlying engine.
- Reuse proven open-source technologies.
- Avoid excessive architectural complexity.
- Separate character creation from animation and rendering.
- Make automated projects editable.
- Never silently replace missing assets with placeholders.
- Prioritize stability, recoverability, and predictable output.
- Preserve user-created work.

---

## 2. Project Scope

### 2.1 Minimum Viable Product (MVP)

The first working version must support:

- Desktop application with a functional 3D viewport
- Importing a rigged GLB character
- Basic character transformations
- An asset library
- A scene editor
- Basic camera controls
- Reusable animation clips
- A simple animation timeline
- JSON scene loading, validation, and execution
- One-character and multi-character scenes
- Offline rendering
- MP4 export
- Saving and reopening projects
- A command-line rendering interface
- A complete 15-second demonstration video

The MVP must use existing rigged character assets. Building sophisticated character meshes from scratch is not required for the initial rendering milestone.

### 2.2 Version 1.0 Scope

After the MVP:

- Template-based character creator
- Face and body customization
- Material and color customization
- Reusable character presets
- Pose editing
- Facial expressions and morph targets
- Animation blending
- Basic inverse kinematics
- Multiple camera shots
- Audio tracks
- Subtitle tracks
- Render queue
- 2.5D visual style presets
- 60-second cartoon generation
- Import/export improvements

### 2.3 Later Versions

- Freeform mesh editing
- Sculpting
- Advanced UV editing
- Procedural character generation
- Automatic rigging
- Animation retargeting
- Lip synchronization
- Physics simulations
- Particle effects
- Node-based materials
- Advanced nonlinear video editor
- Optional AI character creation
- Optional plugin and scripting system
- Optional distributed rendering

These are not MVP requirements.

---

## 3. Technology Stack

| Component | Technology |
|---|---|
| Core application | Rust |
| Desktop interface | egui + eframe |
| GPU rendering | wgpu |
| Math and transformations | glam |
| Data serialization | Serde / serde_json |
| Model import/export | glTF / GLB |
| Native project format | Versioned JSON + assets |
| Offline final rendering (initial) | Blender Python adapter |
| Video/audio encoding | FFmpeg |
| Automated testing | Cargo test |
| Build system | Cargo workspace |
| Source control | Git |
| Continuous integration | GitHub Actions or equivalent |

### 3.1 Technology Decisions

**Rust** manages project data, asset references, timeline execution, animation scheduling, validation, application state, and rendering orchestration.

**egui/eframe** provides the desktop interface.

**wgpu** provides real-time graphics rendering and viewport integration.

**Blender** is initially used as an external offline-rendering backend. Users should not need to manually operate Blender for ordinary exports.

**FFmpeg** handles video and audio encoding, multiplexing, and final MP4 generation.

Blender and FFmpeg must be treated as separately managed dependencies. Their executables, supported versions, availability, and licensing requirements must be checked.

### 3.2 Architecture Rule

The desktop interface must never contain the primary animation or JSON-processing business logic.

The command-line renderer and desktop application must reuse the same core systems.

### 3.3 Dependency Management

- Lock tested Rust dependency versions.
- Avoid wildcard dependency versions.
- Document supported Blender and FFmpeg versions.
- Use lockfiles for reproducible builds.
- Review licenses before redistribution.
- Avoid introducing multiple libraries with duplicate responsibilities.

---

## 4. System Architecture

The system is divided into eight primary modules.

### 4.1 Desktop Application

Responsibilities:

- Main window and navigation
- 3D viewport
- Property inspectors
- Scene hierarchy
- Asset browser
- Animation timeline
- JSON editor
- Render controls
- Project settings

### 4.2 Project Core

Responsibilities:

- Project creation and loading
- Saving and autosaving
- Project version management
- Relative asset path management
- Object identifiers
- Undo/redo transactions
- Scene graph storage
- Data validation
- Project migration

### 4.3 Character System

Responsibilities:

- Character asset registration
- Model import
- Mesh references
- Materials
- Skeleton data
- Facial morph targets
- Character presets
- Appearance customization
- Character instances

### 4.4 Animation Engine

Responsibilities:

- Clip playback
- Timeline scheduling
- Keyframe interpolation
- Skeleton pose evaluation
- Root motion
- Animation blending
- Animation state management
- Basic IK
- Facial expression animation

### 4.5 Scene Engine

Responsibilities:

- Scene graph
- Object transforms
- Cameras
- Lighting
- Environments
- Object visibility
- Character placement
- Scene transitions
- Animation evaluation at arbitrary frames

### 4.6 JSON Engine

Responsibilities:

- JSON parsing
- Schema validation
- Asset resolution
- Scene compilation
- Action validation
- Timeline construction
- Error reporting
- Batch video generation
- JSON-to-project conversion

### 4.7 Rendering Engine

Responsibilities:

- Real-time viewport rendering
- Scene evaluation
- Frame scheduling
- Rendering progress
- Offline rendering orchestration
- Image sequence output
- Final video encoding

### 4.8 Asset Management

Responsibilities:

- Character asset library
- Animation clip library
- Material library
- Environment library
- Audio assets
- Metadata indexing
- Dependency tracking
- Asset validation
- Missing asset detection

---

## 5. Functional Requirements

### FR-001: Project Management

Users must be able to:

- Create a new project.
- Open an existing project.
- Save a project.
- Save a project under another name.
- Rename projects.
- Configure video resolution and frame rate.
- Configure the project asset directory.
- Recover work from autosave after a crash.
- Import an existing JSON video definition.

**Acceptance:** A project must reopen with its scene, characters, camera, animations, and asset references intact.

### FR-002: 3D Viewport

The viewport must support:

- Camera orbit
- Pan
- Zoom
- Perspective view
- Orthographic view
- Object selection
- Move / rotate / scale controls
- World grid
- Object outlines
- Scene lighting preview
- Character pose preview
- Play/pause animation
- Basic frame statistics

**Acceptance:** A user can import a character, inspect it, transform it, and preview its animation.

### FR-003: Character Creator

The character editor must support:

- Creating characters from predefined templates.
- Selecting character categories.
- Customizing body proportions.
- Customizing face proportions.
- Changing eye size, placement, and colors.
- Changing materials and body colors.
- Adding supported accessories.
- Saving custom presets.
- Reusing characters across projects.
- Previewing character expressions.

Initial categories:

- Human
- Animal
- Fantasy creature

Each category requires approved templates and compatible editable parameters. The software must not claim universal editing capabilities that the underlying character models do not support.

**Acceptance:** A user can create a visibly customized character from a supported template and save it for reuse.

### FR-004: Character Import

Required:

- Import GLB / glTF character assets.
- Read supported meshes and materials.
- Read compatible skeletons.
- Discover animation clips.
- Read supported morph targets.
- Inspect imported character metadata.
- Report unsupported asset features.

**Acceptance:** A known valid rigged GLB character loads with the expected skeleton, materials, and animation clips.

### FR-005: Rigging and Posing

The editor should support:

- Skeleton hierarchy inspection
- Joint selection
- Joint rotation
- Pose reset
- Pose presets
- Bone visibility
- Basic IK controls
- Skeleton compatibility validation

Automatic skeleton creation and advanced skin-weight editing are later features.

**Acceptance:** A supported rigged character can be posed without breaking its skin deformation.

### FR-006: Animation System

Required animation types:

- Idle
- Walk
- Run
- Jump
- Wave
- Turn
- Sit
- Blink
- Smile
- Surprise

Animation availability depends on the imported character or compatible animation library.

The engine must support:

- Animation start time
- Duration
- Playback speed
- Looping
- Basic interpolation
- Animation blending
- Clip scheduling
- Transform keyframes
- Animation track editing

**Acceptance:** A character performs three different actions in one scene with correct timing and visible limb movement.

### FR-007: Facial Animation

Support:

- Morph target discovery
- Named facial expressions
- Expression intensity
- Blinking
- Facial keyframes
- Smooth transitions

Advanced lip synchronization is a later feature.

**Acceptance:** A supported character changes expressions without corrupting its facial geometry.

### FR-008: Scene Creation

Users must be able to:

- Create scenes.
- Add characters.
- Add existing environments.
- Add objects and props.
- Configure backgrounds.
- Change lighting.
- Adjust object positions.
- Save reusable scene presets.
- Duplicate and reorder scenes.

**Acceptance:** A saved scene reopens with all positioned assets and configured lighting.

### FR-009: Camera System

Required:

- Perspective cameras
- Orthographic cameras
- Camera position
- Camera rotation
- Field of view
- Camera target
- Camera keyframes
- Multiple camera shots
- Camera cut scheduling

Later:

- Camera tracking
- Dolly motion presets
- Cinematic path editor
- Camera shake
- Advanced depth-of-field controls

**Acceptance:** JSON can schedule at least two camera shots in one generated video.

### FR-010: 2.5D Cartoon Rendering

Support visual styles such as:

- Orthographic cartoon
- Perspective cartoon
- Flat-color stylization
- Soft lighting
- Toon-style materials
- Simple shadow presets

The application must distinguish between true 2D illustration workflows and 3D scenes rendered with a 2.5D appearance.

**Acceptance:** A 3D character can be rendered using a configured orthographic cartoon visual style.

### FR-011: Timeline Editor

Provide tracks for:

- Character animation
- Character transforms
- Camera shots
- Audio
- Facial expressions
- Scene transitions

Required controls:

- Play
- Pause
- Stop
- Scrub
- Zoom timeline
- Move clips
- Adjust clip duration
- Add/delete keyframes
- Undo/redo

**Acceptance:** Changing a clip's start time changes the generated animation accordingly.

### FR-012: JSON Video Generator

The JSON system must support:

- Project metadata
- Video settings
- Scenes
- Character instances
- Asset identifiers
- Object transforms
- Character animation commands
- Camera settings
- Lighting
- Audio references
- Render options
- Output location

The engine must validate JSON before rendering.

**Acceptance:** A valid JSON file generates an MP4; an invalid file produces specific actionable errors.

### FR-013: Visual/JSON Synchronization

Both editing modes must use the same internal scene model.

Required:

- Load JSON into the visual editor.
- Modify supported scene values visually.
- Save back to JSON.
- Preserve valid supported properties.
- Warn about unsupported JSON extensions.
- Avoid silently discarding data.

**Acceptance:** Loading, saving, and reloading an unchanged project must preserve its semantic scene data.

### FR-014: Audio and Subtitles

Initial support:

- Import audio
- Audio start times
- Background music
- Sound effects
- Volume adjustment
- Basic audio mixing
- Subtitle tracks
- Text overlays

Later:

- Voice recording
- Waveform editing
- Lip synchronization
- Spatial audio
- Advanced audio effects

**Acceptance:** Exported MP4 contains correctly synchronized video and audio.

### FR-015: Export and Rendering

Required:

- MP4 output
- H.264 encoding
- Configurable resolution
- Configurable frame rate
- Render progress
- Render cancellation
- Error logs
- Temporary frame cleanup
- Output verification

Initial resolutions:

- 1280 × 720
- 1920 × 1080
- 1080 × 1920

Initial frame rates:

- 24 FPS
- 30 FPS
- 60 FPS

Additional codecs and formats are later features.

**Acceptance:** A completed render produces a playable video with correct duration, frame rate, resolution, and audio when included.

### FR-016: Command-Line Interface

Planned commands:

- `cartoon-studio new`
- `cartoon-studio validate`
- `cartoon-studio render`
- `cartoon-studio inspect`
- `cartoon-studio list-assets`

Illustrative usage:

`cartoon-studio validate examples/hello_cartoon.json`

`cartoon-studio render examples/hello_cartoon.json --output outputs/demo.mp4`

These commands are requirements, not currently implemented functionality.

**Acceptance:** The CLI can render a project without launching the desktop interface.

---

## 6. JSON Specification

### 6.1 JSON Design Principles

- Human-readable
- Versioned
- Strictly validated
- Reusable
- Predictable
- Compatible with the visual editor
- Independent of operating-system-specific paths

### 6.2 Required Fields

- `schema_version`
- `project`
- `video`
- `assets`
- `scenes`
- `output`

### 6.3 Example Video JSON

    {
      "schema_version": "1.0",
      "project": {
        "name": "Cute Cartoon Demo",
        "seed": 42
      },
      "video": {
        "duration_seconds": 15,
        "fps": 30,
        "width": 1920,
        "height": 1080
      },
      "assets": {
        "characters": {
          "hero": "assets/characters/hero_actions.glb"
        }
      },
      "scenes": [
        {
          "id": "scene_001",
          "start_frame": 0,
          "duration_frames": 450,
          "environment": "forest_day",
          "characters": [
            {
              "id": "hero_01",
              "asset": "hero",
              "position": [0, 0, 0],
              "actions": [
                {
                  "clip": "walk",
                  "start_frame": 0,
                  "duration_frames": 120
                },
                {
                  "clip": "wave",
                  "start_frame": 120,
                  "duration_frames": 60
                },
                {
                  "clip": "jump",
                  "start_frame": 180,
                  "duration_frames": 60
                },
                {
                  "clip": "idle",
                  "start_frame": 240,
                  "duration_frames": 210
                }
              ]
            }
          ],
          "camera": {
            "projection": "perspective",
            "position": [2.8, 1.6, 3.0],
            "target": [0, 0.65, 0]
          }
        }
      ],
      "output": {
        "format": "mp4",
        "path": "outputs/demo.mp4"
      }
    }

This is a **proposed application-specific format**. Implementations must define its schema before production use.

The project seed is reserved for deterministic procedural features. It does not by itself guarantee bit-identical video encodes across different renderers or hardware.

### 6.4 JSON Validation Rules

- Every project must declare a supported schema version.
- Character IDs must be unique within their scope.
- Asset references must resolve.
- Frame numbers must be nonnegative.
- Durations must be positive.
- Scheduled actions must fit within supported timing rules.
- Required animation clips must exist.
- Camera values must be valid.
- Output paths must be permitted.
- Unknown or unsupported properties must produce clear diagnostics.
- Invalid projects must not start rendering.

### 6.5 Timing Conventions

- Use integer frames as the canonical internal timeline unit.
- Convert seconds to frames using the configured frame rate.
- Define animation intervals as start-inclusive and end-exclusive.
- Avoid cumulative floating-point timing drift.
- Validate animation and camera overlap behavior.
- Use consistent timing for audio and video.
- Explicitly distinguish animation playback from movement through world space.

### 6.6 Animation Conflict Rules

The engine must define predictable behavior when commands overlap.

- Nonconflicting animation layers may blend.
- Conflicting base animation clips require a defined blend or priority.
- Simultaneous position tracks must not overwrite each other unpredictably.
- Missing animations must return an error unless an explicit fallback is configured.
- Root motion and explicit movement tracks must not apply duplicate displacement.

Current adapter behavior: scene-relative action intervals are sorted by start
and stop at their exclusive end. Optional `speed` (default `1.0`) changes source
animation time without changing these intervals. Optional `loop` (default
`true`) wraps the clip; `false` holds the last pose after one playback.
`blend_in_frames` (default `0`) provides a smooth incoming crossfade measured
in timeline frames, independent of speed. An overlap must exactly equal the
incoming fade duration; undeclared overlaps, three-way overlaps, and overlapping
scenes are rejected. Missing clips error before rendering unless that action sets
`"fallback_clip": "idle"` (the fallback itself must exist). Clips use exact,
case-sensitive names. Character placement is applied once on a parent object.
See [adapter usage and verification](adapters/blender/README.md).


### 6.7 JSON Versioning

- Start with schema version `1.0`.
- Validate the version on load.
- Reject unsupported major versions.
- Introduce documented migrations for older formats.
- Never destroy unknown future-version data silently.
- Provide meaningful migration errors.

---

## 7. Character Data Model

Each character asset must have:

- Unique ID
- Display name
- Character category
- Model path
- Skeleton metadata
- Material references
- Available animation clips
- Available expression names
- Thumbnail
- Supported editing parameters
- Asset version

### Character Instance

A character instance holds scene-specific properties:

- Instance ID
- Asset ID
- Position
- Rotation
- Scale
- Visibility
- Active animations
- Current expression
- Scene-specific customization

Multiple scene instances may share one character asset without duplicating the original model.

### Character Templates

Initial customization is template-based.

A supported template may expose sliders for:

- Head size
- Eye size
- Body width
- Limb length
- Facial proportions
- Color selections
- Accessory visibility

These controls require suitable underlying mesh, skeleton, or morph-target support.

The application must not promise arbitrary modeling from a single reference image in the MVP.

---

## 8. Desktop UI and UX Requirements

### 8.1 Main Application Layout

The interface includes:

**Top toolbar**

- New
- Open
- Save
- Undo
- Redo
- Import
- Preview
- Render

**Left sidebar**

- Project hierarchy
- Scene objects
- Asset browser
- Character library

**Center**

- 3D viewport
- Object transformation tools
- Character preview

**Right sidebar**

- Properties
- Materials
- Character customization
- Animation settings
- Camera and lighting settings

**Bottom panel**

- Animation timeline
- Playback controls
- Frame indicators
- Render progress

### 8.2 Workspace Modes

Provide dedicated modes:

1. Character Studio
2. Scene Studio
3. Animation Studio
4. JSON Studio
5. Render Studio

These modes must share one active project instead of operating as separate disconnected applications.

### 8.3 UX Principles

- Show essential controls first.
- Keep advanced controls in expandable panels.
- Avoid excessive toolbars and icons.
- Provide clear selection feedback.
- Support keyboard shortcuts.
- Make errors understandable.
- Always show whether the project has unsaved changes.
- Provide visible render progress and cancellation.
- Offer beginner-friendly character presets.
- Avoid blocking the interface during long operations.

### 8.4 Default Workflow

1. Open Cartoon Studio.
2. Create a project.
3. Import or create a character.
4. Add a scene.
5. Select character animations.
6. Configure the camera.
7. Preview the animation.
8. Adjust the timeline.
9. Export the video.

### 8.5 JSON Workflow

1. Open JSON Studio.
2. Load a JSON scene definition.
3. Validate the JSON.
4. Resolve assets.
5. Preview the compiled scene.
6. Edit visually if needed.
7. Render the video.
8. Review logs and output.

---

## 9. Project Structure

Use a Rust Cargo workspace.

    cartoon-studio/
    ├── README.md
    ├── Cargo.toml
    ├── Cargo.lock
    ├── rust-toolchain.toml
    ├── .gitignore
    ├── .github/
    │   └── workflows/
    │       └── ci.yml
    ├── apps/
    │   ├── desktop/
    │   └── cli/
    ├── crates/
    │   ├── studio-core/
    │   ├── studio-assets/
    │   ├── studio-characters/
    │   ├── studio-animation/
    │   ├── studio-scene/
    │   ├── studio-json/
    │   ├── studio-viewport/
    │   └── studio-render/
    ├── adapters/
    │   └── blender/
    ├── assets/
    │   ├── characters/
    │   ├── animations/
    │   ├── environments/
    │   ├── materials/
    │   └── audio/
    ├── examples/
    │   ├── hello_cartoon.json
    │   └── sample_project/
    ├── schemas/
    │   └── video-v1.schema.json
    ├── tests/
    │   ├── fixtures/
    │   └── integration/
    ├── scripts/
    ├── docs/
    │   ├── architecture.md
    │   ├── json-spec.md
    │   ├── character-format.md
    │   └── development.md
    └── outputs/

### Structure Rules

- No unrelated files in the project root.
- No duplicate rendering pipelines without explicit justification.
- No hardcoded user-specific absolute paths.
- No generated videos committed by default.
- No dependency cache or temporary render files committed.
- No hidden cross-module dependencies.
- No circular dependencies.
- Keep public interfaces small.
- Separate application code from reusable libraries.
- Introduce additional modules only when needed.

The workspace structure is a target architecture, not a requirement to create every module before implementation begins.

---

## 10. Data and Asset Management

### 10.1 Native Project Format

Use a project directory containing:

- Project manifest
- Scene definitions
- Asset references
- Character presets
- Animation definitions
- Rendering configuration

All internal project asset references should use normalized, project-relative paths or stable asset IDs.

### 10.2 Asset Import

Required:

- File type validation
- Asset metadata extraction
- Duplicate detection
- Relative path normalization
- Compatibility warnings
- Missing asset reporting

### 10.3 Coordinate System

Use a canonical scene coordinate convention:

- Right-handed
- Y-axis up
- Meters for distance

The Blender export adapter must convert coordinates appropriately because Blender uses a different default up-axis convention.

### 10.4 Asset Library

An asset must be reusable without copying its contents into every scene.

Characters, environments, materials, and animations must have stable identifiers.

### 10.5 Asset Licensing

- Record licensing information for bundled assets.
- Do not bundle commercial or restricted assets without permission.
- Respect model, texture, music, and font licenses.
- Review Blender and FFmpeg license obligations before distributing installers.

---

## 11. Rendering Pipeline

### 11.1 Pipeline Stages

1. Load project.
2. Parse and validate scene data.
3. Resolve asset dependencies.
4. Compile the scene timeline.
5. Evaluate animation.
6. Prepare rendering data.
7. Render frames.
8. Process audio.
9. Encode video.
10. Validate final output.
11. Clean temporary resources.

### 11.2 Preview Rendering

Use the native GPU viewport for interactive preview.

Initial supported features:

- Basic meshes
- Materials
- Lighting
- Cameras
- Rigged character playback
- Frame navigation

### 11.3 Final Rendering

Initially use a Blender offline-rendering adapter.

Responsibilities:

- Create or load a compatible Blender scene.
- Import resolved assets.
- Apply character transforms.
- Evaluate animation at scheduled frames.
- Configure camera and lighting.
- Render the required image sequence.
- Return status and errors.

### 11.4 Video Encoding

FFmpeg handles:

- Combining frames
- Adding audio
- Encoding H.264
- Writing MP4
- Checking completion status

### 11.5 Render Reliability

Required:

- Render job IDs
- Progress reporting
- Cancel support
- Separate temporary directories per job
- Error logs
- Output verification
- Cleanup after failure
- Atomic final output replacement where possible

The system must not label a render successful before the output has been validated.

### 11.6 Preview/Final Consistency

The preview renderer and Blender renderer may differ visually.

The application must document supported feature parity and avoid promising pixel-identical output.

Rendering compatibility tests must cover cameras, object transforms, animation timing, materials, and visibility.

---

## 12. Nonfunctional Requirements

### NFR-001: Performance

Initial targets on a supported development machine:

- Responsive UI during normal editing
- Interactive viewport for simple scenes
- Smooth playback of basic imported characters
- No unnecessary scene recompilation for unrelated property changes
- Rendering operations must not freeze the UI
- Avoid unnecessary repeated asset loading

Viewport FPS and render time must be measured with representative benchmark scenes before defining production guarantees.

### NFR-002: Reliability

- Avoid project corruption.
- Support autosave.
- Support crash recovery where practical.
- Keep render outputs separate from project source files.
- Preserve original assets during imports and transformations.
- Provide safe project migrations.

### NFR-003: Maintainability

- Modular Rust workspace
- Clear module ownership
- Automated formatting
- Linting
- Unit tests
- Integration tests
- Documented public APIs
- Minimal unnecessary abstractions

### NFR-004: Portability

Target platforms:

1. macOS — initial development target
2. Windows — second validation target
3. Linux — additional validation target

Cross-platform compatibility must influence architecture from the beginning, but formal platform support requires completed platform testing.

### NFR-005: Security

- Reject asset paths escaping permitted project locations.
- Validate imported files.
- Avoid executing commands embedded in JSON.
- Do not accept arbitrary shell commands as animation actions.
- Launch Blender and FFmpeg with controlled arguments.
- Restrict output paths appropriately.
- Handle malformed and oversized input safely.
- Keep external execution logs.
- Do not transmit projects to remote services without explicit authorization.

### NFR-006: Usability

- Clear error messages
- Undo/redo support
- Accessible keyboard navigation where practical
- Basic onboarding
- Tooltips
- Sample project
- Documented controls

### NFR-007: Compatibility

- Versioned project schema
- Versioned video JSON schema
- Explicit asset compatibility rules
- Tested Blender adapter versions
- Supported OS/GPU matrix

---

## 13. Development Roadmap

Development follows working vertical slices rather than building every module simultaneously.

### Phase 0 — Discovery and Foundation

**Estimated effort:** 1–2 developer-weeks

Tasks:

- Review any existing reusable video-generator source.
- Identify dependencies and ownership.
- Decide which components to reuse.
- Initialize Rust workspace.
- Establish code formatting and linting.
- Establish CI.
- Create the first desktop application shell.
- Create the CLI shell.
- Define the internal project model.
- Write architecture decisions.

Deliverables:

- Clean repository
- Compiling Rust workspace
- Basic desktop window
- Initial CLI
- Architecture documentation

**Exit criteria:** Workspace builds and tests run successfully.

### Phase 1 — 3D Viewport and Asset Import

**Estimated effort:** 3–5 developer-weeks

Tasks:

- Initialize GPU rendering.
- Add viewport camera controls.
- Import GLB assets.
- Display meshes and materials.
- Add object hierarchy.
- Add object selection.
- Add transform controls.
- Implement basic lighting.

Deliverables:

- Interactive 3D viewport
- Imported character visible
- Basic object editing

**Exit criteria:** A rigged GLB can be loaded and inspected in the application.

### Phase 2 — Scene and JSON Engine

**Estimated effort:** 3–5 developer-weeks

Tasks:

- Define the versioned JSON schema.
- Implement parsing and validation.
- Build scene graph.
- Add stable asset IDs.
- Resolve character references.
- Add timeline scheduling.
- Add camera definitions.
- Create sample JSON files.
- Implement CLI validation.

Deliverables:

- Functional JSON validation
- Loaded scene model
- Useful diagnostics

**Exit criteria:** JSON compiles into a valid internal scene with correctly resolved assets.

### Phase 3 — Animation and Video Generation

**Estimated effort:** 4–7 developer-weeks

Tasks:

- Load animation clips.
- Evaluate skeleton animation.
- Add frame-based timing.
- Schedule walk, wave, jump, and idle actions.
- Add camera cuts.
- Implement Blender render adapter.
- Integrate FFmpeg.
- Add render progress and logs.
- Implement CLI rendering.

Deliverables:

- Working video-generation pipeline
- 15-second MP4 example
- Automated rendering test

**Exit criteria:** A JSON file produces a playable 15-second animation with correct character movement.

**This is the first major release milestone.**

### Phase 4 — Character Studio

**Estimated effort:** 5–9 developer-weeks

Tasks:

- Create supported character templates.
- Add appearance presets.
- Add parameter sliders.
- Add material customization.
- Add facial customization.
- Add accessories.
- Add character preview controls.
- Implement preset saving and reloading.

Deliverables:

- Working template-based character creator
- Reusable custom character assets

**Exit criteria:** Users can customize, save, reopen, and animate a supported character.

### Phase 5 — Visual Animation Editor

**Estimated effort:** 5–9 developer-weeks

Tasks:

- Implement timeline UI.
- Add animation clip editing.
- Add frame scrubbing.
- Add transform keyframes.
- Add pose tools.
- Add basic animation blending.
- Add expression controls.
- Add multiple camera tracks.
- Synchronize visual editing and JSON data.

Deliverables:

- Editable animation timeline
- Character posing
- JSON-to-visual editing workflow

**Exit criteria:** Users can open a JSON-generated animation, edit its timing visually, and export the updated result.

### Phase 6 — Audio, Rendering UX, and Longer Videos

**Estimated effort:** 3–6 developer-weeks

Tasks:

- Add music and sound effects.
- Add audio timing.
- Add basic subtitles.
- Add render queue.
- Add cancellation and recovery.
- Add longer video tests.
- Add vertical video presets.
- Improve render diagnostics.

Deliverables:

- Audio-capable MP4 generation
- Render queue
- 60-second example project

**Exit criteria:** A 60-second cartoon with multiple actions, camera shots, and audio renders successfully.

### Phase 7 — Quality, Documentation, and Release

**Estimated effort:** 3–5 developer-weeks

Tasks:

- Run full automated test suite.
- Test supported operating systems.
- Profile runtime and memory.
- Fix major UI issues.
- Check asset licenses.
- Validate installer dependencies.
- Create sample projects.
- Write user documentation.
- Package installers.

Deliverables:

- Release candidate
- User documentation
- Sample assets and sample projects
- Known limitations
- Installation instructions

**Exit criteria:** All release-blocking acceptance tests pass.

### Overall Effort

Initial indicative effort:

- First JSON-rendered cartoon milestone: approximately 11–19 developer-weeks
- Broader version 1.0: approximately 27–48 developer-weeks

These are planning estimates, not delivery commitments. Actual effort depends heavily on graphics experience, available character assets, platform issues, and required animation quality.

---

## 14. Testing Strategy

### 14.1 Unit Tests

Test:

- JSON parsing
- Schema validation
- Frame calculations
- Transform math
- Asset identifiers
- Scene graph behavior
- Timeline overlap rules
- Animation scheduling
- Path resolution
- Project migration

### 14.2 Integration Tests

Test:

- Character import
- Project save/reload
- JSON scene compilation
- Animation clip resolution
- Rendering adapter invocation
- FFmpeg encoding
- Output metadata validation

### 14.3 End-to-End Tests

Required scenarios:

1. Create a project and save it.
2. Import a rigged character.
3. Preview a character animation.
4. Load a JSON animation.
5. Render a 15-second MP4.
6. Render a 60-second MP4.
7. Open a project containing multiple characters.
8. Edit a JSON-generated timeline visually.
9. Reopen a saved project without missing data.
10. Recover appropriately from an interrupted render.

### 14.4 Negative Tests

Test:

- Invalid JSON
- Missing assets
- Missing animation clips
- Invalid camera configuration
- Unsupported character file
- Unwritable output location
- Corrupted project data
- Unsupported schema version
- Render cancellation
- Blender/FFmpeg unavailable
- Unsafe file paths

### 14.5 Visual Validation

Automated tests cannot fully evaluate cartoon quality.

Review representative renders for:

- Character appearance
- Mesh deformation
- Foot contact
- Movement smoothness
- Facial expressions
- Camera framing
- Lighting consistency
- Animation continuity
- Scene composition

---

## 15. Quality Gates

### Gate A — Foundation

- [ ] Workspace builds successfully.
- [ ] UI starts.
- [ ] CLI starts.
- [ ] CI checks pass.

### Gate B — Character Preview

- [ ] GLB imports.
- [ ] Materials display.
- [ ] Camera controls work.
- [ ] Object transforms work.
- [ ] Skeleton data is recognized.

### Gate C — JSON Animation

- [ ] JSON schema validates.
- [ ] Assets resolve.
- [ ] Animation timing is correct.
- [ ] Character movement is visible.
- [ ] Camera settings work.

### Gate D — Video Export

- [ ] Frames render.
- [ ] MP4 encodes successfully.
- [ ] Output metadata is correct.
- [ ] No unexpected blank frames.
- [ ] Errors are logged.
- [ ] Temporary files are managed.

### Gate E — Character Creator

- [ ] Template loads.
- [ ] Customization controls work.
- [ ] Preset saves.
- [ ] Preset reopens.
- [ ] Rig and animation remain compatible.

### Gate F — Release

- [ ] Project recovery works.
- [ ] Main workflows are documented.
- [ ] Platform tests pass.
- [ ] Third-party license requirements are reviewed.
- [ ] No release-blocking defects remain.

---

## 16. Definition of Done

A task is complete only when:

- The implementation matches the documented requirement.
- It builds successfully.
- Relevant tests pass.
- Existing functionality is not broken.
- Errors are handled.
- No unnecessary duplicate code is introduced.
- Documentation is updated when required.
- The feature has been manually verified if visual behavior is involved.

A character is not considered fully rigged merely because its entire model can move.

An animation is not considered complete if only whole-character bouncing occurs when articulated movement is required.

A render is not considered successful unless a playable output file is verified.

---

## 17. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Character modeling becomes too complex | Start with customizable templates |
| Rendering engine takes too long | Reuse Blender for offline rendering |
| Preview differs from export | Maintain rendering compatibility tests |
| Rigging produces poor deformation | Use validated rigs and test poses |
| Animation clips are incompatible | Add explicit skeleton compatibility checks |
| JSON becomes hard to maintain | Versioned schema and strict validation |
| Large project files become messy | Stable asset library and clear module boundaries |
| Rendering fails late | Preflight validation |
| Rust graphics integration becomes complex | Prototype viewport before major development |
| Scope becomes unmanageable | Enforce milestones and defer advanced features |

---

## 18. Future Enhancements

### 18.1 Advanced Modeling

- Edit mesh vertices
- Edit edges and faces
- Subdivision tools
- Sculpt brushes
- Retopology assistance
- UV unwrapping

### 18.2 Advanced Animation

- Automatic rigging
- Skeleton retargeting
- Full-body IK
- Motion paths
- Animation graphs
- Physics-assisted motion

### 18.3 AI-Assisted Creation

Optional future capabilities:

- Text-to-character concepts
- Image-guided character creation
- Text-to-animation assistance
- Automated scene planning
- Dialogue-to-lip-sync
- Assisted animation cleanup

AI output must be validated before being treated as production-ready assets.

### 18.4 Advanced Video Editing

- Multi-scene editing
- Video clip import
- Transitions
- Advanced compositing
- Color grading
- Effects
- Export presets
- Multiple audio tracks

### 18.5 Extensibility

- Plugin API
- Scripting API
- Reusable JSON templates
- Batch rendering
- Asset marketplace compatibility
- Optional cloud rendering

---

## 19. Initial Development Backlog

### Priority P0 — Must Build First

- [ ] Set up Rust workspace
- [ ] Create desktop application shell
- [ ] Create CLI
- [ ] Implement project data model
- [ ] Implement JSON validation
- [ ] Implement GLB import
- [ ] Create functional 3D viewport
- [ ] Load rigged character animations
- [ ] Implement scene timeline
- [ ] Integrate offline renderer
- [ ] Integrate FFmpeg
- [ ] Render first 15-second MP4

### Priority P1 — Character and Editing Tools

- [ ] Character presets
- [ ] Body and face customization
- [ ] Material customization
- [ ] Character pose controls
- [ ] Timeline editor
- [ ] Facial expressions
- [ ] Camera shot editing
- [ ] Audio and subtitles
- [ ] 60-second rendering

### Priority P2 — Advanced Features

- [ ] Freeform mesh modeling
- [ ] Advanced rigging
- [ ] Automatic rigging
- [ ] Lip synchronization
- [ ] Advanced 2.5D styles
- [ ] Visual effects
- [ ] Advanced video editing
- [ ] Optional AI features

---

## 20. Development Rules for Codex and AI Coding Agents

1. Read this README before implementing features.
2. Understand the existing code before modifying it.
3. Follow the current development phase.
4. Do not implement future-phase features without approval.
5. Reuse existing modules when suitable.
6. Avoid unnecessary architecture rewrites.
7. Keep files organized by responsibility.
8. Never delete important assets without a verified replacement.
9. Write tests for important logic.
10. Run available tests after modifications.
11. Report exactly what changed.
12. Clearly distinguish completed, partial, and unsupported features.
13. Never use placeholder behavior to falsely satisfy acceptance tests.
14. Verify rendered output for visual features.
15. Keep the project buildable at the end of each implementation step.
16. Prefer a small, complete feature over many incomplete features.
17. Do not introduce hardcoded paths or secrets.
18. Update documentation when architecture or behavior changes.

### Expected AI Agent Report

After each implementation task, provide:

- Summary of changes
- Files added or modified
- Features completed
- Tests executed and results
- Render artifacts, if applicable
- Known limitations
- Next recommended task

---

## 21. First Development Milestone

**Milestone:** Hello Cartoon

### Objective

Create a working desktop application that loads one rigged 3D character and produces a 15-second cartoon video using a JSON definition.

### Required Demo

The demonstration must include:

- One imported rigged character
- One environment
- A visible 3D viewport
- Idle, walk, wave, and jump animation clips
- At least one camera
- JSON-defined animation timing
- MP4 output
- Rendering logs
- Valid output metadata

### Success Conditions

- Application launches.
- JSON validates.
- Character imports correctly.
- Actions execute in the expected order.
- Character limbs visibly animate.
- The camera correctly frames the character.
- The renderer completes without errors.
- Exported MP4 opens and plays.
- The original project remains intact.

The next development phase must not begin until this vertical slice is working reliably.

---

## 22. Reference Documentation

- Rust: https://www.rust-lang.org/
- egui: https://docs.rs/egui/
- eframe: https://docs.rs/eframe/
- wgpu: https://wgpu.rs/
- Serde: https://serde.rs/
- glTF: https://www.khronos.org/gltf/
- Blender: https://docs.blender.org/
- FFmpeg: https://ffmpeg.org/documentation.html

---

## 23. Final Product Goal

Cartoon Studio should eventually allow a user to:

1. Create a character visually.
2. Customize its appearance.
3. Rig or import a compatible rig.
4. Add reusable animations.
5. Place characters in a scene.
6. Create an animation using a timeline or JSON.
7. Preview the complete cartoon.
8. Render an MP4 video.
9. Save the project for later editing.
10. Reuse characters and animation assets in future projects.

**The core product promise: Create characters visually. Animate manually or through JSON. Render complete cartoons from one unified studio.**

---

**Project Status:** Specification only. Development has not started under this plan.# toonvara
