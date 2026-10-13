# Blender Adapter

`render.py` imports a GLB, assigns the case-sensitive `TOONVARA_CLIP` action to
the first armature, disables imported NLA animation, and renders PNGs. It binds
the imported action slot on Blender versions with layered actions. This direct
single-clip mode remains available for diagnostics. The CLI uses JSON scheduling
as described below.

Both `TOONVARA_FRAME_START` and `TOONVARA_FRAME_END` select an inclusive range
in the clip's native timeline. `TOONVARA_OUT_PATH` is a filename prefix in this
mode. Without either frame bound it is an exact PNG filename, and a selected
clip renders a still at its first frame. Clips are not looped or retimed; frames
beyond the clip hold its endpoint. `TOONVARA_FPS` defaults to 24 and is applied
before GLB import so animation seconds map to the requested frame rate.

From the repository root, render the included fox's `Walk` clip (native frames
0–17 at 24 fps). Its model units require the larger camera distance below:

```sh
mkdir -p target/clip-verification
TOONVARA_CHARACTER_GLB="$PWD/examples/assets/characters/hero.glb" \
TOONVARA_CLIP=Walk \
TOONVARA_FPS=24 \
TOONVARA_FRAME_START=1 TOONVARA_FRAME_END=17 \
TOONVARA_OUT_PATH="$PWD/target/clip-verification/walk_" \
TOONVARA_RES_X=320 TOONVARA_RES_Y=240 \
TOONVARA_CAM_POS_X=200 TOONVARA_CAM_POS_Y=110 TOONVARA_CAM_POS_Z=220 \
TOONVARA_CAM_TGT_Y=35 \
blender --background --python-exit-code 1 --python adapters/blender/render.py
```

This writes `walk_0001.png` through `walk_0017.png`. Multi-frame clip renders
compare parent-relative deform-bone matrices at up to five evenly spaced
frames, logging changed bones as `TOONVARA_BONES`. They also log SHA-256 hashes
of those exact PNG paths as `TOONVARA_HASH`. Stale files are excluded. A sample
with no relative bone motion or no differing PNG hashes fails with an error;
choose a range that includes motion. Hashes alone cannot establish articulation.

Locally verified with Blender 5.2.2: frames 1, 5, 9, 13, and 17 had five distinct
hashes, and 20 bones changed relative to their parents, including forearms,
legs, feet, spine, head, and tail.

The CLI resolves every character asset and sends all scenes to `schedule.py`
through `TOONVARA_SCHEDULE_PATH` (a temporary JSON manifest). JSON scheduling
takes precedence over `TOONVARA_CLIP`. Video FPS is applied before GLB import;
the video duration determines the total frame count, including scene gaps.

- Scene starts are global; action starts are relative to the scene.
- JSON intervals are `[start, start + duration)`. JSON frame 0 is PNG frame 1.
- `speed` is a finite positive multiplier, default `1.0`; zero, negative,
  nonfinite, null, boolean, and string speeds are rejected. All positive finite
  numbers are accepted, not just the four demonstration presets.
- `loop` defaults to `true` for compatibility. Source time wraps with fractional
  frame interpolation. `false` clamps to the final pose, holding it through the
  scheduled end. Slow clips can be cut off by the scheduled end; fast clips do
  not shorten the action. Speed affects root motion and bones together.
- `blend_in_frames` defaults to `0`. Explicit two-clip overlaps crossfade with
  a smoothstep weight on the incoming NLA track. The overlap must exactly equal
  its fade duration, which must be shorter than the action. Fade duration stays
  in timeline frames regardless of speed. Without an outgoing clip, it fades
  from the underlying rest pose.
- Action order is sorted by start, independently for each character instance.
- Undeclared/three-way action overlaps, overlapping scenes, zero durations, actions outside their scene,
  scenes outside the video, duplicate IDs, and unresolved assets error.
- Missing clips error before any frames render. An action may explicitly use
  `"fallback_clip": "idle"`; the named fallback must exist. No silent aliases.
- All imported animations are disabled before scheduling. Parent transforms
  place each character once; clip root motion remains inside that placement.
- Gaps have no active animation strip; scene visibility uses the same exclusive
  end rule. Cameras switch at scene boundaries. Expressions, environments,
  camera projection/FOV, movement tracks, additive layers, and animation masks
  remain rough or unsupported. Only the first armature in each GLB receives its clips.

The example now references `hero_actions.glb`, preserving the original fox in
`hero.glb`. `build_demo_actions.py` reproducibly creates the new asset using the
original walk and simple authored wave, jump (root travel plus limb motion),
and subtle idle clips. They demonstrate scheduling, not polished animation.
The wider camera keeps the jump in view. Rebuild the demo asset with:

```sh
blender --background --python-exit-code 1 --python adapters/blender/build_demo_actions.py
```

Render all 450 example frames at preview resolution through the existing
FFmpeg encoder, then verify actual decoded MP4 frames:

```sh
cargo run -p toonvara-cli -- render examples/hello_cartoon.json \
  --width 640 --height 360 --out target/schedule-verification/hello_cartoon.mp4
blender --background --python-exit-code 1 --python adapters/blender/verify_schedule_video.py
```

Expected order: walk 0–4s, wave 4–6s, jump 6–8s, idle 8–15s. The CLI saves a
`.render.log` beside the MP4 with resolved clips and intervals. Verification
checks 450 decoded frames/30 fps/15 seconds, compares 13 decoded samples with
independent single-clip renders (including transitions and the final frame),
and checks that interior samples match their own clip more closely than the
other three clips. It writes `verification/verification.json` and review PNGs.

Timeline regression tests compare scheduled bones against direct clip poses
at interval boundaries and inside all four clips, and exercise missing clips,
fallbacks, overlaps, and multiple instances of the same asset:

```sh
blender --background --python-exit-code 1 --python adapters/blender/test_schedule.py
```

Run the Blender regression suite (real PNGs, clip isolation, still output,
invalid clips/ranges, held poses, and rejection of object-only bouncing):

```sh
blender --background --python-exit-code 1 --python adapters/blender/test_render.py
```

`inspect_glb.py` reports imported armatures, bones, and available clip names;
set `TOONVARA_GLB_PATH` and invoke it using the same Blender arguments.

See `crates/studio-render/src/blender.rs` for the Rust-side orchestration.

Per-action speed example (existing JSON needs no changes):

```json
{
  "clip": "wave",
  "start_frame": 120,
  "duration_frames": 90,
  "speed": 1.5,
  "loop": true
}
```

For a crossfade from an outgoing action ending at frame 150, add
`"blend_in_frames": 30` to this incoming action starting at 120. Both actions
retain their original intervals and playback clocks. The incoming clip has
zero influence at 120 and full influence at 150, exactly when the outgoing
clip becomes inactive. Missing-clip fallbacks inherit speed/loop/fade options.

Speed mapping is computed from absolute elapsed timeline time on every frame
(including Blender subframes), never by incrementing a counter. Looping maps
`elapsed * speed` into the native source interval; one-shots clamp at its end.
The implementation reduces the elapsed time before multiplication to avoid
overflow for extreme finite speeds. It does not round fractional source frames,
stretch the scheduled interval, or use speed to alter blending weights.

Loop continuity depends on the source animation having a matching seam. The
adapter preserves authored interpolation and does not synthesize missing
transition poses or accumulate root travel across repeated cycles. Very high
speeds can alias at a fixed output FPS. Reverse playback and zero-speed pause
are intentionally not supported.

Render and verify the 15-second speed demonstration:

```sh
cargo run -p toonvara-cli -- render examples/speed_demo.json
blender --background --python-exit-code 1 --python adapters/blender/verify_speed_video.py
```

The demo uses the same `wave` clip at 0.5x (0–3s), 1x (3–6s), 1.5x (6–9s), and
2x (9–12s). A non-looping 2x jump runs from 12–15s, reaches its end at 13s, then
holds. Verification checks 450 frames/30 fps/15 seconds, matches 35 decoded
frames to independently rendered source poses (including fractional samples),
rejects incorrect 1x reference poses for changed speeds, and checks the final
hold. Results and sample PNGs go in `target/speed-verification/verification/`.
A captioned MP4 is also written with the original video stream copied unchanged;
players supporting subtitles show each speed and looping mode.

`test_schedule.py` also covers all four speeds in looping/non-looping modes,
exact activation boundaries, cyclic seam continuity, fractional source poses,
extreme speeds, and actual skeletal mixing between differently paced clips.
