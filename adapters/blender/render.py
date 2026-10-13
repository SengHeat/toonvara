"""Toonvara Blender adapter — render frames.

This script is executed inside Blender via:
    blender --background --python render.py

It reads parameters from environment variables (set by the Rust caller) so
that no user-controlled data ever passes through shell interpolation.

Environment variables
---------------------
TOONVARA_OUT_PATH       Output path.  For a single frame this is the full PNG
                        path.  For a sequence this is the *prefix* — Blender
                        appends the zero-padded frame number and extension
                        (e.g.  prefix -> prefix0001.png).
TOONVARA_RES_X          Output width in pixels  (default 1920).
TOONVARA_RES_Y          Output height in pixels (default 1080).
TOONVARA_CAM_POS_X/Y/Z  Camera position (default 0, 2, 6).
TOONVARA_CAM_TGT_X/Y/Z  Camera look-at target  (default 0, 1, 0).
TOONVARA_FRAME_START    First frame number (default: not set -> single frame).
TOONVARA_FRAME_END      Last frame number  (default: not set -> single frame).
TOONVARA_CHARACTER_GLB  Path to a .glb character file to import (optional).
                        When set, the character replaces the fallback cube.
TOONVARA_CHAR_POS_X/Y/Z Character position in Y-up coords (default 0,0,0).
TOONVARA_CLIP           Animation clip (action) name to play on the armature
                        (optional, case-sensitive). Uses the clip's native frame
                        numbers; no looping or JSON action scheduling. Without
                        an explicit range, renders a still at the clip start.
TOONVARA_FPS            Scene FPS, set before import (default 24).
TOONVARA_SCHEDULE_PATH  Resolved scene manifest from the Rust pipeline. When
                        set, schedules JSON actions instead of TOONVARA_CLIP.
"""

import os
import sys
import math
import hashlib
from pathlib import Path

import bpy                       # type: ignore[import-untyped]
from mathutils import Vector     # type: ignore[import-untyped]


# ---------------------------------------------------------------------------
# Read parameters from environment
# ---------------------------------------------------------------------------

def env_float(name: str, default: float) -> float:
    raw = os.environ.get(name, "")
    if raw == "":
        return default
    return float(raw)


def env_int(name: str, default: int) -> int:
    raw = os.environ.get(name, "")
    if raw == "":
        return default
    return int(raw)


out_path = os.environ.get("TOONVARA_OUT_PATH", "")
if not out_path:
    print("ERROR: TOONVARA_OUT_PATH not set", file=sys.stderr)
    sys.exit(1)

res_x = env_int("TOONVARA_RES_X", 1920)
res_y = env_int("TOONVARA_RES_Y", 1080)

cam_pos = Vector((
    env_float("TOONVARA_CAM_POS_X", 0.0),
    # Blender is Z-up; our convention is Y-up.  Swap Y <-> Z.
    env_float("TOONVARA_CAM_POS_Z", 6.0),
    env_float("TOONVARA_CAM_POS_Y", 2.0),
))

cam_tgt = Vector((
    env_float("TOONVARA_CAM_TGT_X", 0.0),
    env_float("TOONVARA_CAM_TGT_Z", 0.0),
    env_float("TOONVARA_CAM_TGT_Y", 1.0),
))

# Frame range — if both are set we render an animation sequence.
frame_start_raw = os.environ.get("TOONVARA_FRAME_START", "")
frame_end_raw   = os.environ.get("TOONVARA_FRAME_END", "")
sequence_mode   = bool(frame_start_raw and frame_end_raw)
frame_start     = int(frame_start_raw) if frame_start_raw else 1
frame_end       = int(frame_end_raw)   if frame_end_raw   else 1
if bool(frame_start_raw) != bool(frame_end_raw) or frame_end < frame_start:
    raise ValueError("provide both frame bounds with start <= end")

# Optional GLB character import.
character_glb = os.environ.get("TOONVARA_CHARACTER_GLB", "")

# Character position (Y-up -> Z-up conversion, same as camera).
char_pos = Vector((
    env_float("TOONVARA_CHAR_POS_X", 0.0),
    env_float("TOONVARA_CHAR_POS_Z", 0.0),
    env_float("TOONVARA_CHAR_POS_Y", 0.0),
))

# Optional animation clip to play.
clip_name = os.environ.get("TOONVARA_CLIP", "")
schedule_path = os.environ.get("TOONVARA_SCHEDULE_PATH", "")


# ---------------------------------------------------------------------------
# Build the scene
# ---------------------------------------------------------------------------

# Wipe the default scene.
bpy.ops.wm.read_factory_settings(use_empty=True)

scene = bpy.context.scene
fps = env_int("TOONVARA_FPS", 24)
if fps < 1:
    raise ValueError("TOONVARA_FPS must be positive")
scene.render.fps = fps

# --- Subject (GLB character or fallback cube) -----------------------------

if schedule_path:
    sys.path.insert(0, str(Path(__file__).parent))
    import schedule
    schedule.build(schedule_path, scene)
    clip_name = ""  # The JSON schedule takes precedence over a single clip.
elif character_glb:
    # Import the GLB character.
    if not os.path.isfile(character_glb):
        print(f"ERROR: character GLB not found: {character_glb}", file=sys.stderr)
        sys.exit(1)

    print(f"TOONVARA: importing character from {character_glb}")
    bpy.ops.import_scene.gltf(filepath=character_glb)

    # Collect every root-level object that was just imported.
    imported = list(scene.objects)

    # Move imported roots to the requested position.
    for obj in imported:
        if obj.parent is None:
            obj.location += char_pos

    print(f"TOONVARA: imported {len(imported)} object(s)")
else:
    # Fallback: default cube.
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0.5))
    cube = bpy.context.active_object
    cube.name = "ToonvaraCube"

    mat = bpy.data.materials.new(name="CubeMat")
    if hasattr(mat, 'use_nodes'):
        mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf is not None:
        bsdf.inputs["Base Color"].default_value = (0.2, 0.6, 0.9, 1.0)
    cube.data.materials.append(mat)

# --- Print scene object inventory (for log verification) ------------------
print("TOONVARA: scene objects:")
for obj in bpy.context.scene.objects:
    parent_name = obj.parent.name if obj.parent else "(root)"
    print(f"  - {obj.name}  type={obj.type}  parent={parent_name}")

# --- Armature summary (for model identification) --------------------------
for obj in bpy.context.scene.objects:
    if obj.type == 'ARMATURE':
        bones = [b.name for b in obj.data.bones]
        print(f"TOONVARA_RIG:{obj.name}:{len(bones)} bones:{', '.join(bones[:8])}"
              + (f" ... +{len(bones)-8} more" if len(bones) > 8 else ""))

# --- Animation clip assignment --------------------------------------------

if clip_name:
    # Find the action by name.
    action = bpy.data.actions.get(clip_name)
    if action is None:
        available = [a.name for a in bpy.data.actions]
        print(f"ERROR: clip '{clip_name}' not found. "
              f"Available: {available}", file=sys.stderr)
        sys.exit(1)

    # Find the first armature in the scene.
    armature_obj = None
    for obj in bpy.context.scene.objects:
        if obj.type == 'ARMATURE':
            armature_obj = obj
            break

    if armature_obj is None:
        print("ERROR: no armature found to assign animation clip", file=sys.stderr)
        sys.exit(1)

    # The glTF importer retains other clips in NLA tracks and may activate a
    # different action. Disable all imported animation before binding ONE clip.
    # Remember the matching strip's slot: layered actions (Blender 4.4+) need
    # that slot as well as the action, or the rig can silently remain static.
    slot = None
    for obj in imported:
        data = obj.animation_data
        if data is None:
            continue
        for track in data.nla_tracks:
            for strip in track.strips:
                if obj == armature_obj and strip.action == action:
                    slot = getattr(strip, "action_slot", None)
        data.action = None
        data.use_nla = False
    data = armature_obj.animation_data_create()
    data.action = action
    if hasattr(action, "slots") and action.slots:
        if slot is None:
            slot = next((s for s in action.slots
                         if s.target_id_type == 'OBJECT'), None)
        if slot is None:
            raise ValueError(f"clip '{clip_name}' has no armature action slot")
        data.action_slot = slot
    data.action_blend_type = 'REPLACE'
    data.action_influence = 1.0

    # Keep the caller's explicit range in the clip's native timeline.
    clip_start = math.floor(action.frame_range[0])
    clip_end   = math.ceil(action.frame_range[1])
    if sequence_mode:
        # Caller specified explicit range — respect it but log the clip range.
        print(f"TOONVARA: playing clip '{clip_name}' "
              f"(clip frames {clip_start}-{clip_end}, "
              f"rendering frames {frame_start}-{frame_end})")
    else:
        # The single-frame Rust API expects exactly out_path, not a sequence.
        frame_start = clip_start
        print(f"TOONVARA: clip '{clip_name}', still at frame {frame_start}")

    # Compare parent-relative deform-bone matrices. Object/root translation
    # alone cannot satisfy this check (nor can whole-body bouncing).
    sample_frames = sorted({frame_start + (frame_end - frame_start) * i // 4
                            for i in range(5)}) if sequence_mode else [frame_start]
    poses = []
    for frame in sample_frames:
        scene.frame_set(frame)
        poses.append({
            bone.name: bone.parent.matrix.inverted_safe() @ bone.matrix
            for bone in armature_obj.pose.bones
            if bone.parent is not None and bone.bone.use_deform
        })
    moving_bones = sorted(name for name, initial in poses[0].items()
                          if any(max(abs(pose[name][r][c] - initial[r][c])
                                     for r in range(4) for c in range(4)) > 1e-5
                                 for pose in poses[1:]))
    if sequence_mode and frame_end > frame_start:
        if not moving_bones:
            raise RuntimeError("selected clip/range has no relative deform-bone motion")
        print(f"TOONVARA_BONES:{','.join(moving_bones)}")
    scene.frame_set(frame_start)

# --- Camera ---------------------------------------------------------------
cam_data = bpy.data.cameras.new(name="ToonvaraCamera")
cam_data.lens = 50  # mm
cam_obj = bpy.data.objects.new("ToonvaraCamera", cam_data)
scene.collection.objects.link(cam_obj)
scene.camera = cam_obj

cam_obj.location = cam_pos
direction = cam_tgt - cam_pos
rot_quat = direction.to_track_quat('-Z', 'Y')
cam_obj.rotation_euler = rot_quat.to_euler()

# --- Sun light ------------------------------------------------------------
light_data = bpy.data.lights.new(name="ToonvaraSun", type='SUN')
light_data.energy = 3.0
light_obj = bpy.data.objects.new("ToonvaraSun", light_data)
light_obj.rotation_euler = (math.radians(50), math.radians(10), math.radians(-30))
scene.collection.objects.link(light_obj)

# --- World background -----------------------------------------------------
world = bpy.data.worlds.new("ToonvaraWorld")
if hasattr(world, 'use_nodes'):
    world.use_nodes = True
bg = world.node_tree.nodes.get("Background")
if bg is not None:
    bg.inputs["Color"].default_value = (0.05, 0.05, 0.08, 1.0)
scene.world = world


# ---------------------------------------------------------------------------
# Render settings
# ---------------------------------------------------------------------------

# Blender 4.x used BLENDER_EEVEE_NEXT; 5.x renamed it back to BLENDER_EEVEE.
try:
    scene.render.engine = 'BLENDER_EEVEE_NEXT'
except TypeError:
    scene.render.engine = 'BLENDER_EEVEE'

scene.render.resolution_x = res_x
scene.render.resolution_y = res_y
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'
scene.render.filepath = out_path

# Keep it fast — this is a test frame.
scene.render.film_transparent = False
if hasattr(scene, 'eevee') and hasattr(scene.eevee, 'taa_render_samples'):
    scene.eevee.taa_render_samples = 16


# ---------------------------------------------------------------------------
# Render
# ---------------------------------------------------------------------------

if sequence_mode:
    scene.frame_start = frame_start
    scene.frame_end = frame_end
    scene.frame_set(frame_start)
    print(f"TOONVARA: rendering frames {frame_start}-{frame_end} "
          f"({res_x}x{res_y}) to {out_path}")
    bpy.ops.render.render(animation=True)
    print(f"TOONVARA: sequence render complete ({frame_end - frame_start + 1} frames)")
else:
    print(f"TOONVARA: rendering {res_x}x{res_y} to {out_path}")
    bpy.ops.render.render(write_still=True)
    print("TOONVARA: render complete")


# ---------------------------------------------------------------------------
# Frame hash verification (for animation correctness)
# ---------------------------------------------------------------------------

if sequence_mode and clip_name:
    # Use exact paths for this range, never stale frames from a prefix glob.
    if len(sample_frames) >= 2:
        hashes = set()
        for frame in sample_frames:
            f = scene.render.frame_path(frame=frame)
            with open(f, "rb") as fh:
                h = hashlib.sha256(fh.read()).hexdigest()
                hashes.add(h)
            print(f"TOONVARA_HASH:{os.path.basename(f)}:{h}")
        if len(hashes) > 1:
            print(f"TOONVARA: animation verified — {len(hashes)} distinct frames in sample")
        else:
            raise RuntimeError("all sampled PNGs are identical despite the selected clip")
