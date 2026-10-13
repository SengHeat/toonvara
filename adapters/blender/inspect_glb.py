"""Toonvara Blender adapter — inspect a GLB file.

Imports a GLB and prints structured metadata about armatures, bones, and
animation clips.  Does NOT render anything.

Executed via:
    blender --background --python inspect_glb.py

Environment variables
---------------------
TOONVARA_GLB_PATH   Absolute path to the .glb file to inspect (required).

Output format (on stdout, one per line)
---------------------------------------
TOONVARA_INSPECT:armature:<name>:<bone_count>
TOONVARA_INSPECT:bone:<armature>:<bone_name>
TOONVARA_INSPECT:action:<name>:<frame_start>:<frame_end>
TOONVARA_INSPECT:mesh_count:<N>
TOONVARA_INSPECT:object_count:<N>
TOONVARA_INSPECT:done
"""

import os
import sys

import bpy  # type: ignore[import-untyped]

glb_path = os.environ.get("TOONVARA_GLB_PATH", "")
if not glb_path:
    print("ERROR: TOONVARA_GLB_PATH not set", file=sys.stderr)
    sys.exit(1)

if not os.path.isfile(glb_path):
    print(f"ERROR: file not found: {glb_path}", file=sys.stderr)
    sys.exit(1)

# Wipe default scene and import.
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=glb_path)

# --- Object / mesh counts ------------------------------------------------
objects = list(bpy.context.scene.objects)
mesh_count = sum(1 for o in objects if o.type == 'MESH')
print(f"TOONVARA_INSPECT:object_count:{len(objects)}")
print(f"TOONVARA_INSPECT:mesh_count:{mesh_count}")

# --- Armatures and bones -------------------------------------------------
for obj in objects:
    if obj.type == 'ARMATURE' and obj.data:
        bone_count = len(obj.data.bones)
        print(f"TOONVARA_INSPECT:armature:{obj.name}:{bone_count}")
        for bone in obj.data.bones:
            print(f"TOONVARA_INSPECT:bone:{obj.name}:{bone.name}")

# --- Animation clips (actions + NLA tracks) -------------------------------
# Collect from bpy.data.actions (all actions loaded from the GLB).
for action in bpy.data.actions:
    f_start, f_end = action.frame_range
    print(f"TOONVARA_INSPECT:action:{action.name}:{f_start:.0f}:{f_end:.0f}")

# Also scan NLA tracks on every object for any pushed-down strips that
# might reference additional actions not in bpy.data.actions directly.
seen_actions = {a.name for a in bpy.data.actions}
for obj in objects:
    if obj.animation_data and obj.animation_data.nla_tracks:
        for track in obj.animation_data.nla_tracks:
            for strip in track.strips:
                if strip.action and strip.action.name not in seen_actions:
                    f_start, f_end = strip.action.frame_range
                    print(f"TOONVARA_INSPECT:action:{strip.action.name}:{f_start:.0f}:{f_end:.0f}")
                    seen_actions.add(strip.action.name)

print("TOONVARA_INSPECT:done")
