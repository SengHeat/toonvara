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
                        (e.g.  prefix → prefix0001.png).
TOONVARA_RES_X          Output width in pixels  (default 1920).
TOONVARA_RES_Y          Output height in pixels (default 1080).
TOONVARA_CAM_POS_X/Y/Z  Camera position (default 0, 2, 6).
TOONVARA_CAM_TGT_X/Y/Z  Camera look-at target  (default 0, 1, 0).
TOONVARA_FRAME_START    First frame number (default: not set → single frame).
TOONVARA_FRAME_END      Last frame number  (default: not set → single frame).
"""

import os
import sys
import math

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


# ---------------------------------------------------------------------------
# Build the scene
# ---------------------------------------------------------------------------

# Wipe the default scene.
bpy.ops.wm.read_factory_settings(use_empty=True)

scene = bpy.context.scene

# --- Default cube ---------------------------------------------------------
bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0.5))
cube = bpy.context.active_object
cube.name = "ToonvaraCube"

# Simple material so the cube isn't pitch black.
mat = bpy.data.materials.new(name="CubeMat")
if hasattr(mat, 'use_nodes'):
    mat.use_nodes = True
bsdf = mat.node_tree.nodes.get("Principled BSDF")
if bsdf is not None:
    bsdf.inputs["Base Color"].default_value = (0.2, 0.6, 0.9, 1.0)
cube.data.materials.append(mat)

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
    print(f"TOONVARA: rendering frames {frame_start}-{frame_end} "
          f"({res_x}x{res_y}) to {out_path}")
    bpy.ops.render.render(animation=True)
    print(f"TOONVARA: sequence render complete ({frame_end - frame_start + 1} frames)")
else:
    print(f"TOONVARA: rendering {res_x}x{res_y} to {out_path}")
    bpy.ops.render.render(write_still=True)
    print("TOONVARA: render complete")
