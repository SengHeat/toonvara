"""Blender timeline tests: --python-exit-code 1 --python test_schedule.py."""

import copy
import json
import math
from pathlib import Path
import sys
import tempfile
import unittest

import bpy

sys.path.insert(0, str(Path(__file__).parent))
import schedule

ROOT = Path(__file__).resolve().parents[2]


class ScheduleTests(unittest.TestCase):
    def setUp(self):
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.context.scene.render.fps = 30
        self.definition = json.loads((ROOT / 'examples/hello_cartoon.json').read_text())['scenes']
        self.character = self.definition[0]['characters'][0]
        self.character['asset'] = str(ROOT / 'examples/assets/characters/hero_actions.glb')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)

    def build(self):
        path = Path(self.temp.name) / 'schedule.json'
        path.write_text(json.dumps(self.definition))
        return schedule.build(path, bpy.context.scene)

    def test_exact_boundaries_and_native_playback(self):
        self.character['actions'].reverse()  # File ordering must not define timing.
        records = self.build()
        self.assertEqual([r['clip'] for r in records], ['walk', 'wave', 'jump', 'idle'])
        rig = next(obj for obj in bpy.context.scene.objects if obj.type == 'ARMATURE')
        data = rig.animation_data
        strips = [track.strips[0] for track in data.nla_tracks]
        self.assertEqual([(r['start'], r['end']) for r in records],
                         [(0, 120), (120, 180), (180, 240), (240, 450)])
        # At each boundary and inside every interval, scheduled poses must match
        # a direct evaluation of the intended source clip at the native time.
        for index, strip in enumerate(strips):
            for offset in (0, 15, records[index]['end'] - records[index]['start'] - 1):
                with self.subTest(clip=records[index]['clip'], offset=offset):
                    bpy.context.scene.frame_set(records[index]['start'] + offset + 1)
                    expected = {b.name: b.matrix.copy() for b in rig.pose.bones}
                    position = rig.location.copy()
                    data.use_nla = False
                    data.action = strip.action
                    if hasattr(strip, 'action_slot'):
                        data.action_slot = strip.action_slot
                    source = strip.action_frame_start + offset % (strip.action_frame_end - strip.action_frame_start)
                    bpy.context.scene.frame_set(int(source), subframe=source % 1)
                    delta = max(abs(b.matrix[r][c] - expected[b.name][r][c])
                                for b in rig.pose.bones for r in range(4) for c in range(4))
                    location_delta = (rig.location - position).length
                    if delta >= 0.002:
                        print('MISMATCH', strip.name, offset, strip.scale, strip.repeat,
                              sorted((max(abs(b.matrix[r][c] - expected[b.name][r][c]) for r in range(4) for c in range(4)), b.name) for b in rig.pose.bones)[-3:])
                    data.action = None
                    data.use_nla = True
                    self.assertLess(delta, 0.002)
                    self.assertLess(location_delta, 0.002)

    def test_missing_clip_errors_and_explicit_fallback_works(self):
        self.character['actions'][1]['clip'] = 'missing'
        with self.assertRaisesRegex(ValueError, "missing clip 'missing'"):
            self.build()
        bpy.ops.wm.read_factory_settings(use_empty=True)
        self.character['actions'][1]['fallback_clip'] = 'idle'
        records = self.build()
        self.assertEqual(records[1]['clip'], 'idle')
        self.assertEqual(records[1]['requested'], 'missing')

    def test_missing_fallback_errors(self):
        self.character['actions'][0].update(clip='missing', fallback_clip='also-missing')
        with self.assertRaisesRegex(ValueError, 'also-missing'):
            self.build()

    def test_overlap_and_out_of_bounds_rejected(self):
        for start in (119, 449):
            bpy.ops.wm.read_factory_settings(use_empty=True)
            self.character['actions'][1]['start_frame'] = start
            with self.assertRaisesRegex(ValueError, 'invalid action interval'):
                self.build()

    def test_two_characters_reusing_asset_and_scene_offset(self):
        self.definition[0]['start_frame'] = 10
        second = copy.deepcopy(self.character)
        second['id'] = 'hero_02'
        second['position'] = [2, 0, 0]
        second['actions'] = [dict(clip='wave', start_frame=0, duration_frames=450)]
        self.definition[0]['characters'].append(second)
        records = self.build()
        self.assertEqual(records[-1]['clip'], 'wave')
        rigs = [obj for obj in bpy.context.scene.objects if obj.type == 'ARMATURE']
        self.assertEqual(len(rigs), 2)
        self.assertTrue(all(r.animation_data.nla_tracks[0].strips[0].frame_start == 10 for r in rigs))
        bpy.context.scene.frame_set(10)
        self.assertTrue(all(r.hide_render for r in rigs))
        bpy.context.scene.frame_set(11)
        self.assertTrue(all(not r.hide_render for r in rigs))
        bpy.context.scene.frame_set(461)
        self.assertTrue(all(r.hide_render for r in rigs))

    def assert_source_pose(self, rig, strip, timeline, source):
        scene = bpy.context.scene
        scene.frame_set(math.floor(timeline + 1), subframe=(timeline + 1) % 1)
        expected = {bone.name: bone.matrix.copy() for bone in rig.pose.bones}
        position = rig.location.copy()
        data = rig.animation_data
        data.use_nla = False
        data.action = strip.action
        if hasattr(strip, 'action_slot'):
            data.action_slot = strip.action_slot
        scene.frame_set(math.floor(source), subframe=source % 1)
        error = max(abs(bone.matrix[r][c] - expected[bone.name][r][c])
                    for bone in rig.pose.bones for r in range(4) for c in range(4))
        position_error = (rig.location - position).length
        data.action = None
        data.use_nla = True
        self.assertLess(error, 0.002)
        self.assertLess(position_error, 0.002)

    def test_speeds_looping_and_one_shots_match_fractional_source_poses(self):
        for speed in (0.5, 1, 1.5, 2):
            for looping in (False, True):
                with self.subTest(speed=speed, looping=looping):
                    bpy.ops.wm.read_factory_settings(use_empty=True)
                    bpy.context.scene.render.fps = 30
                    self.character['actions'] = [dict(clip='jump', start_frame=10, duration_frames=90,
                                                     speed=speed, loop=looping)]
                    records = self.build()
                    self.assertEqual((records[0]['start'], records[0]['end']), (10, 100))
                    rig = next(obj for obj in bpy.context.scene.objects if obj.type == 'ARMATURE')
                    strip = rig.animation_data.nla_tracks[0].strips[0]
                    for elapsed in (0, 0.5, 1, 15, 29.5, 30, 59, 60, 89):
                        source = elapsed * speed
                        source = source % 60 if looping else min(source, 60)
                        self.assert_source_pose(rig, strip, 10 + elapsed, source)
                    bpy.context.scene.frame_set(10)  # before inclusive start
                    self.assertTrue(strip.mute)
                    bpy.context.scene.frame_set(11)
                    self.assertFalse(strip.mute)
                    bpy.context.scene.frame_set(101)  # exclusive end
                    self.assertTrue(strip.mute)

    def test_loop_seam_is_continuous_for_cyclic_clip(self):
        self.character['actions'] = [dict(clip='wave', start_frame=0, duration_frames=450, speed=1.5)]
        self.build()
        rig = next(obj for obj in bpy.context.scene.objects if obj.type == 'ARMATURE')
        # The source wave has matching endpoints; sample either side of three
        # wraps, including fractional times, to catch snapping or quantization.
        for cycle in (1, 2, 5):
            poses = []
            for offset in (-0.0001, 0.0001):
                frame = 1 + cycle * 40 + offset
                bpy.context.scene.frame_set(math.floor(frame), subframe=frame % 1)
                poses.append({b.name: b.matrix.copy() for b in rig.pose.bones})
            error = max(abs(poses[0][b][r][c] - poses[1][b][r][c])
                        for b in poses[0] for r in range(4) for c in range(4))
            self.assertLess(error, 0.01)

    def test_crossfade_uses_timeline_weights_and_independent_source_speeds(self):
        self.character['actions'] = [
            dict(clip='wave', start_frame=0, duration_frames=90, speed=0.5),
            dict(clip='idle', start_frame=60, duration_frames=90, speed=2, blend_in_frames=30),
        ]
        self.build()
        rig = next(obj for obj in bpy.context.scene.objects if obj.type == 'ARMATURE')
        outgoing, incoming = [track.strips[0] for track in rig.animation_data.nla_tracks]
        bpy.context.scene.frame_set(76)  # timeline frame 75: half-way through fade
        self.assertAlmostEqual(outgoing.strip_time, 37.5)
        self.assertAlmostEqual(incoming.strip_time, 30)
        self.assertAlmostEqual(incoming.influence, 0.5)
        mixed = {b.name: b.matrix.copy() for b in rig.pose.bones}
        for strip in (incoming, outgoing):
            strip.mute = True
            bpy.context.view_layer.update()
            error = max(abs(b.matrix[r][c] - mixed[b.name][r][c])
                        for b in rig.pose.bones for r in range(4) for c in range(4))
            self.assertGreater(error, 0.01)  # both clips actually affect the pose
            strip.mute = False
        bpy.context.scene.frame_set(61)
        self.assertAlmostEqual(incoming.influence, 0)
        self.assertFalse(outgoing.mute)
        bpy.context.scene.frame_set(91)
        self.assertAlmostEqual(incoming.influence, 1)
        self.assertTrue(outgoing.mute)
        self.assertFalse(incoming.mute)

    def test_invalid_playback_options_rejected(self):
        for speed in (0, -1, float('nan'), float('inf'), -float('inf'), 'fast', True, None):
            with self.subTest(speed=speed), self.assertRaisesRegex(ValueError, 'speed'):
                schedule.playback_options(dict(speed=speed, duration_frames=60))
        for options in (dict(loop=1), dict(blend_in_frames=-1), dict(blend_in_frames=60)):
            with self.assertRaises(ValueError):
                schedule.playback_options(dict(duration_frames=60, **options))
        self.character['actions'][0]['speed'] = 0
        with self.assertRaisesRegex(ValueError, 'speed'):
            self.build()

    def test_extreme_finite_speeds_do_not_overflow_source_time(self):
        for speed in (1e-300, 1e300):
            for looping in (True, False):
                source = schedule.playback_time(400, 0, 60, speed, looping)
                self.assertTrue(math.isfinite(source))
                self.assertGreaterEqual(source, 0)
                self.assertLessEqual(source, 60)


result = unittest.main(argv=[__file__], exit=False)
if not result.result.wasSuccessful():
    raise RuntimeError('Schedule regression tests failed')
