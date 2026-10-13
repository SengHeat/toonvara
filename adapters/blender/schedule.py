"""Build scheduled NLA clips for each JSON character instance."""

import json
import math
from pathlib import Path

import bpy
from mathutils import Euler


def playback_options(item):
    speed = item.get('speed', 1.0)
    looping = item.get('loop', True)
    blend = item.get('blend_in_frames', 0)
    if isinstance(speed, bool) or not isinstance(speed, (int, float)) or not math.isfinite(speed) or speed <= 0:
        raise ValueError('action speed must be finite and greater than zero')
    if not isinstance(looping, bool):
        raise ValueError('action loop must be a boolean')
    if type(blend) is not int or not 0 <= blend < item['duration_frames']:
        raise ValueError('blend_in_frames must be a nonnegative integer less than duration_frames')
    return speed, looping, blend


def configure_blend(strip, start, duration):
    """A smoothstep fade in timeline frames, independent of the source clock."""
    if duration == 0:
        return
    strip.use_animated_influence = True
    # Enabling animated influence creates a zero-valued key automatically.
    # Replace it with a timeline curve; assigning strip.influence in a handler
    # would be overwritten by Blender's evaluation of that default key.
    curve = next(curve for curve in strip.fcurves if curve.data_path == 'influence')
    curve.keyframe_points.clear()
    for frame, value in ((start + 1, 0), (start + duration + 1, 1)):
        point = curve.keyframe_points.insert(frame, value)
        point.interpolation = 'BEZIER'
        point.handle_left_type = point.handle_right_type = 'FREE'
        point.handle_left = (frame - duration / 3, value)
        point.handle_right = (frame + duration / 3, value)
    curve.extrapolation = 'CONSTANT'
    curve.update()


def action_bindings(objects):
    """Track names preserve glTF clip names even on a repeated asset import."""
    rig = next((obj for obj in objects if obj.type == 'ARMATURE'), None)
    bindings = {}
    if rig and rig.animation_data:
        for track in rig.animation_data.nla_tracks:
            for strip in track.strips:
                if strip.action:
                    bindings[track.name] = (strip.action, getattr(strip, 'action_slot', None))
    return rig, bindings


def disable_imported_animation(objects):
    for obj in objects:
        if obj.animation_data:
            obj.animation_data.action = None
            for track in list(obj.animation_data.nla_tracks):
                obj.animation_data.nla_tracks.remove(track)


def build(path, scene):
    scenes = json.loads(Path(path).read_text())
    instances = []
    records = []
    previous_scene_end = 0
    for definition in sorted(scenes, key=lambda item: item['start_frame']):
        scene_start = definition['start_frame']
        scene_end = scene_start + definition['duration_frames']
        if scene_start < previous_scene_end or scene_end <= scene_start:
            raise ValueError('overlapping scenes or invalid scene interval')
        previous_scene_end = scene_end
        for character in definition['characters']:
            glb_path = character['asset']
            print(f"TOONVARA_IMPORT:{character['id']}:{glb_path}")
            before = set(scene.objects)
            bpy.ops.import_scene.gltf(filepath=glb_path)
            objects = list(set(scene.objects) - before)
            rig, bindings = action_bindings(objects)
            if rig:
                bone_count = len(rig.data.bones)
                bone_sample = ', '.join(b.name for b in list(rig.data.bones)[:6])
                print(f"TOONVARA_ARMATURE:{rig.name}:{bone_count}:{bone_sample}")
                print(f"TOONVARA_CLIPS:{sorted(bindings)}")
            disable_imported_animation(objects)
            container = bpy.data.objects.new(character['id'], None)
            scene.collection.objects.link(container)
            for obj in objects:
                if obj.parent is None:
                    obj.parent = container
            x, y, z = character.get('position', [0, 0, 0])
            container.location = (x, z, y)
            x, y, z = character.get('scale', [1, 1, 1])
            container.scale = (x, z, y)
            x, y, z = character.get('rotation', [0, 0, 0])
            container.rotation_euler = Euler(tuple(math.radians(v) for v in (x, z, y)))
            instances.append((objects, character.get('visible', True), scene_start, scene_end))
            actions = sorted(character.get('actions', []), key=lambda item: item['start_frame'])
            if actions and rig is None:
                raise ValueError(f"{character['id']}: no armature for scheduled actions")
            if not actions:
                continue
            data = rig.animation_data_create()
            data.use_nla = True
            previous_end = 0
            earlier_end = 0
            for item in actions:
                speed, looping, blend = playback_options(item)
                start = item['start_frame']
                end = start + item['duration_frames']
                overlap = start < previous_end
                if (end <= start or end > definition['duration_frames'] or
                        (overlap and (previous_end - start != blend or start < earlier_end))):
                    raise ValueError(f"{character['id']}: overlapping or invalid action interval")
                earlier_end = previous_end
                previous_end = end
                name = item['clip']
                if name not in bindings:
                    name = item.get('fallback_clip')
                if name not in bindings:
                    raise ValueError(f"{character['id']}: missing clip '{item['clip']}' "
                                     f"(fallback {item.get('fallback_clip')!r}); available: {sorted(bindings)}")
                action_obj, slot = bindings[name]
                native_start, native_end = action_obj.frame_range
                if native_end <= native_start:
                    raise ValueError(f"clip '{name}' must contain at least two keyed frames")

                source_len = native_end - native_start
                duration = end - start  # JSON timeline frames

                # JSON frame 0 maps to Blender frame 1.
                # Strips placed at their actual timeline position — NLA handles
                # repeat and scale natively; no handler-driven strip_time.
                blender_start = scene_start + start + 1
                blender_end = scene_start + end + 1

                track = data.nla_tracks.new()
                track.name = f'Toonvara {start}: {name}'
                strip = track.strips.new(name, blender_start, action_obj)
                if slot is not None:
                    strip.action_slot = slot
                strip.action_frame_start = native_start
                strip.action_frame_end = native_end
                strip.blend_type = 'REPLACE'
                strip.use_auto_blend = False
                strip.blend_in = strip.blend_out = 0

                # Speed: NLA scale = 1/speed (scale > 1 slows down).
                strip.scale = 1.0 / speed

                if looping and source_len > 0:
                    # Fill the timeline slot; Blender loops the action.
                    strip.repeat = max(1.0, (duration * speed) / source_len)
                    strip.extrapolation = 'NOTHING'
                else:
                    # Play once, hold last frame for any remaining time.
                    strip.repeat = 1.0
                    strip.extrapolation = 'HOLD'

                # Lock frame bounds after setting scale/repeat.  Setting
                # frame_end last lets Blender keep our exact bounds while
                # adjusting the effective repeat to match.
                strip.frame_start = blender_start
                strip.frame_end = blender_end

                configure_blend(strip, scene_start + start, blend)

                record = dict(scene=definition['id'], character=character['id'], requested=item['clip'],
                              clip=name, start=scene_start + start, end=scene_start + end,
                              source_start=native_start, source_end=native_end,
                              speed=speed, loop=looping, blend_in_frames=blend)
                records.append(record)
                print('TOONVARA_SCHEDULE:' + json.dumps(record, sort_keys=True))

    # Visibility and camera switching still need a frame-change handler.
    def update_visibility(current_scene):
        frame = current_scene.frame_current + current_scene.frame_subframe - 1
        for objects, visible, start, end in instances:
            for obj in objects:
                obj.hide_render = not (visible and start <= frame < end)
        for definition in scenes:
            if definition['start_frame'] <= frame < definition['start_frame'] + definition['duration_frames']:
                camera = definition.get('camera')
                if camera and current_scene.camera:
                    from mathutils import Vector
                    x, y, z = camera['position']
                    current_scene.camera.location = (x, z, y)
                    x, y, z = camera['target']
                    direction = Vector((x, z, y)) - current_scene.camera.location
                    current_scene.camera.rotation_euler = direction.to_track_quat('-Z', 'Y').to_euler()
                break

    bpy.app.handlers.frame_change_pre.append(update_visibility)
    return records
