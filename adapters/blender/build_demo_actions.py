"""Rebuild hero_actions.glb from the original fox; run inside Blender.

Keeps the original Walk and authors deliberately simple wave/jump/idle demos.
The original hero.glb is never modified. No downloaded animations are used.
"""

import math
from pathlib import Path

import bpy
from mathutils import Quaternion


assets = Path(__file__).resolve().parents[2] / 'examples/assets/characters'
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.fps = 30
bpy.ops.import_scene.gltf(filepath=str(assets / 'hero.glb'))
rig = next(obj for obj in scene.objects if obj.type == 'ARMATURE')
for obj in list(scene.objects):
    if obj != rig and obj.parent != rig:
        bpy.data.objects.remove(obj, do_unlink=True)
rig.animation_data.action = bpy.data.actions['Walk']
if hasattr(rig.animation_data, 'action_slot'):
    rig.animation_data.action_slot = rig.animation_data.action.slots[0]
rig.animation_data.use_nla = False
scene.frame_set(0)
base = {bone.name: bone.matrix_basis.copy() for bone in rig.pose.bones}
walk = rig.animation_data.action
walk.name = 'walk'
walk.use_fake_user = True
for track in list(rig.animation_data.nla_tracks):
    rig.animation_data.nla_tracks.remove(track)
rig.scale = (0.01, 0.01, 0.01)


def author(name, duration, offsets, height=lambda t: 0):
    rig.animation_data.action = None
    for frame in range(duration + 1):
        t = frame / duration
        rig.location.z = height(t)
        rig.keyframe_insert('location', frame=frame, group='Root')
        for bone in rig.pose.bones:
            bone.matrix_basis = base[bone.name]
            bone.rotation_mode = 'QUATERNION'
            if bone.name in offsets:
                axis, angle = offsets[bone.name](t)
                bone.rotation_quaternion = bone.rotation_quaternion @ Quaternion(axis, angle)
            bone.keyframe_insert('location', frame=frame, group=bone.name)
            bone.keyframe_insert('rotation_quaternion', frame=frame, group=bone.name)
            bone.keyframe_insert('scale', frame=frame, group=bone.name)
    action = rig.animation_data.action
    action.name = name
    action.use_fake_user = True
    return action


wave = author('wave', 60, {
    'b_LeftUpperArm_09': lambda t: ((1, 0, 0), -1.2 * math.sin(math.pi * t)),
    'b_LeftForeArm_010': lambda t: ((0, 0, 1), math.sin(math.pi * t) * (0.8 + 0.5 * math.sin(6 * math.pi * t))),
    'b_Head_05': lambda t: ((0, 1, 0), 0.15 * math.sin(2 * math.pi * t)),
})
jump = author('jump', 60, {
    name: (lambda t: ((1, 0, 0), 0.65 * math.sin(2 * math.pi * t)))
    for name in ['b_LeftUpperArm_09', 'b_RightUpperArm_06', 'b_LeftLeg01_015', 'b_RightLeg01_019']
}, height=lambda t: 0.55 * math.sin(math.pi * t) ** 2)
idle = author('idle', 90, {
    'b_Tail01_012': lambda t: ((0, 1, 0), 0.10 * math.sin(2 * math.pi * t)),
    'b_Neck_04': lambda t: ((1, 0, 0), 0.025 * math.sin(2 * math.pi * t)),
})
rig.animation_data.action = None
for action in list(bpy.data.actions):
    if action not in (walk, wave, jump, idle):
        bpy.data.actions.remove(action)
for action in (walk, wave, jump, idle):
    track = rig.animation_data.nla_tracks.new()
    track.name = action.name
    strip = track.strips.new(action.name, 0, action)
    if hasattr(action, 'slots'):
        strip.action_slot = action.slots[0]
    track.mute = True
rig.location = (0, 0, 0)
scene.frame_start = 0
scene.frame_end = 90
bpy.ops.export_scene.gltf(filepath=str(assets / 'hero_actions.glb'), export_format='GLB')
