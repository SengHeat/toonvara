"""Blender integration regressions; run with --python-exit-code 1 --python.

Requires examples/assets/characters/hero.glb (Walk, Run, Survey).
"""

import contextlib
import hashlib
import io
import os
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import patch

import bpy


ADAPTER = Path(__file__).with_name("render.py")
HERO = ADAPTER.parents[2] / "examples/assets/characters/hero.glb"


class ClipRenderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="toonvara_clips_")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def render(self, **overrides):
        env = {k: v for k, v in os.environ.items() if not k.startswith("TOONVARA_")}
        env.update({
            "TOONVARA_CHARACTER_GLB": str(HERO),
            "TOONVARA_CLIP": "Walk",
            "TOONVARA_FRAME_START": "1",
            "TOONVARA_FRAME_END": "5",
            "TOONVARA_OUT_PATH": str(self.directory / "frame_"),
            "TOONVARA_RES_X": "160",
            "TOONVARA_RES_Y": "120",
            "TOONVARA_CAM_POS_X": "200",
            "TOONVARA_CAM_POS_Y": "110",
            "TOONVARA_CAM_POS_Z": "220",
            "TOONVARA_CAM_TGT_Y": "35",
        })
        env.update(overrides)
        log = io.StringIO()
        with patch.dict(os.environ, env, clear=True), contextlib.redirect_stdout(log):
            runpy.run_path(str(ADAPTER), run_name="__main__")
        return log.getvalue()

    def test_walk_renders_distinct_frames_and_isolates_action(self):
        # A stale frame must never be included in the adapter's sample hashes.
        (self.directory / "frame_9999.png").write_bytes(b"stale")
        log = self.render()
        frames = [self.directory / f"frame_{i:04d}.png" for i in range(1, 6)]
        self.assertEqual(len({hashlib.sha256(p.read_bytes()).digest() for p in frames}), 5)
        self.assertNotIn("TOONVARA_HASH:frame_9999", log)
        self.assertIn("TOONVARA_BONES:", log)
        rig = next(o for o in bpy.context.scene.objects if o.type == "ARMATURE")
        self.assertEqual(rig.animation_data.action.name, "Walk")
        self.assertFalse(rig.animation_data.use_nla)
        if hasattr(rig.animation_data, "action_slot"):
            self.assertIsNotNone(rig.animation_data.action_slot)

    def test_still_with_clip_preserves_exact_output_path(self):
        output = self.directory / "still.png"
        self.render(TOONVARA_FRAME_START="", TOONVARA_FRAME_END="",
                    TOONVARA_OUT_PATH=str(output))
        self.assertGreater(output.stat().st_size, 0)
        self.assertEqual(list(self.directory.glob("*.png")), [output])

    def test_missing_clip_fails(self):
        with self.assertRaises(SystemExit) as error:
            self.render(TOONVARA_CLIP="does-not-exist")
        self.assertEqual(error.exception.code, 1)

    def test_partial_and_reversed_ranges_fail(self):
        for start, end in [("1", ""), ("5", "1")]:
            with self.subTest(start=start, end=end), self.assertRaises(ValueError):
                self.render(TOONVARA_FRAME_START=start, TOONVARA_FRAME_END=end)

    def test_held_pose_range_fails_motion_check(self):
        with self.assertRaisesRegex(RuntimeError, "no relative deform-bone motion"):
            self.render(TOONVARA_FRAME_START="100", TOONVARA_FRAME_END="104")

    def test_whole_body_bounce_fails_motion_check(self):
        # Export a rig with a deforming child bone, but animate only its object.
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.object.armature_add()
        rig = bpy.context.object
        bpy.ops.object.mode_set(mode="EDIT")
        child = rig.data.edit_bones.new("Child")
        child.head = (0, 0, 1)
        child.tail = (0, 0, 2)
        child.parent = rig.data.edit_bones[0]
        bpy.ops.object.mode_set(mode="OBJECT")
        bpy.ops.mesh.primitive_cube_add()
        mesh = bpy.context.object
        mesh.parent = rig
        modifier = mesh.modifiers.new("Skin", "ARMATURE")
        modifier.object = rig
        group = mesh.vertex_groups.new(name="Child")
        group.add(list(range(len(mesh.data.vertices))), 1.0, "REPLACE")
        for frame, height in [(1, 0), (3, 2), (5, 0)]:
            rig.location.z = height
            rig.keyframe_insert(data_path="location", frame=frame)
        rig.animation_data.action.name = "Bounce"
        bpy.context.scene.frame_start = 1
        bpy.context.scene.frame_end = 5
        fixture = self.directory / "bounce.glb"
        bpy.ops.export_scene.gltf(filepath=str(fixture), export_format="GLB")
        with self.assertRaisesRegex(RuntimeError, "no relative deform-bone motion"):
            self.render(TOONVARA_CHARACTER_GLB=str(fixture), TOONVARA_CLIP="Bounce")


result = unittest.main(argv=[__file__], exit=False)
if not result.result.wasSuccessful():
    raise RuntimeError("Blender animation regression tests failed")
