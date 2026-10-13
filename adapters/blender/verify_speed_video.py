"""Verify examples/speed_demo.json's MP4 and add speed captions without re-encoding.

Run: blender --background --python-exit-code 1 --python adapters/blender/verify_speed_video.py
"""

import json
import math
import os
from pathlib import Path
import runpy
import subprocess
from unittest.mock import patch

import bpy
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
project = json.loads((ROOT / 'examples/speed_demo.json').read_text())
video = ROOT / project['output']['path']
output = video.parent / 'verification'
output.mkdir(parents=True, exist_ok=True)
probe = json.loads(subprocess.check_output([
    'ffprobe', '-v', 'error', '-select_streams', 'v:0', '-count_frames',
    '-show_entries', 'stream=width,height,r_frame_rate,nb_read_frames,duration', '-of', 'json', str(video),
]))['streams'][0]
assert probe['nb_read_frames'] == '450' and probe['r_frame_rate'] == '30/1', probe
assert abs(float(probe['duration']) - 15) < 0.01, probe
records = [json.loads(line.split(':', 1)[1]) for line in video.with_suffix('.render.log').read_text().splitlines()
           if line.startswith('TOONVARA_SCHEDULE:')]
assert [(r['start'], r['end']) for r in records] == [(i * 90, (i + 1) * 90) for i in range(5)]
assert [r['speed'] for r in records] == [0.5, 1, 1.5, 2, 2]
assert [r['loop'] for r in records] == [True, True, True, True, False]


def pixels(path):
    image = bpy.data.images.load(str(path), check_existing=False)
    data = np.asarray(image.pixels[:], dtype=np.float32).reshape(-1, 4)[:, :3].copy()
    bpy.data.images.remove(image)
    return data


# Independently compute expected source times rather than calling the scheduler.
samples = []
for record in records:
    for offset in (0, 1, 17, 30, 41, 61, 89):
        elapsed = offset * record['speed']
        length = record['source_end'] - record['source_start']
        phase = elapsed % length if record['loop'] else min(elapsed, length)
        samples.append(dict(frame=record['start'] + offset, clip=record['clip'],
                            speed=record['speed'], source=record['source_start'] + phase))

decoded = {}
for sample in samples:
    frame = sample['frame']
    path = output / f'decoded_{frame:04d}.png'
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(video), '-vf',
                    f'select=eq(n\\,{frame})', '-frames:v', '1', str(path)], check=True)
    decoded[frame] = pixels(path)

references = {}
camera = project['scenes'][0]['camera']
for clip in ('wave', 'jump'):
    env = {k: v for k, v in os.environ.items() if not k.startswith('TOONVARA_')}
    env.update({
        'TOONVARA_CHARACTER_GLB': str(ROOT / 'examples' / project['assets']['characters']['hero']),
        'TOONVARA_CLIP': clip, 'TOONVARA_FPS': '30',
        'TOONVARA_OUT_PATH': str(output / f'setup_{clip}.png'),
        'TOONVARA_RES_X': str(probe['width']), 'TOONVARA_RES_Y': str(probe['height']),
    })
    for index, axis in enumerate('XYZ'):
        env[f'TOONVARA_CAM_POS_{axis}'] = str(camera['position'][index])
        env[f'TOONVARA_CAM_TGT_{axis}'] = str(camera['target'][index])
    # The direct single-clip path has no scheduling/blend/speed controls.
    with patch.dict(os.environ, env, clear=True):
        runpy.run_path(str(Path(__file__).with_name('render.py')), run_name='__main__')
    for source in sorted({s['source'] for s in samples if s['clip'] == clip} | {17.0}):
        bpy.context.scene.frame_set(math.floor(source), subframe=source % 1)
        path = output / f'reference_{clip}_{source:g}.png'
        bpy.context.scene.render.filepath = str(path)
        bpy.ops.render.render(write_still=True)
        references[(clip, source)] = pixels(path)

for sample in samples:
    error = float(np.abs(decoded[sample['frame']] - references[(sample['clip'], sample['source'])]).mean())
    assert error < 0.01, (sample, error)
    sample['mean_absolute_error'] = error

negative_controls = []
for record in records:
    if record['speed'] == 1:
        continue
    sample = next(s for s in samples if s['frame'] == record['start'] + 17)
    wrong_error = float(np.abs(decoded[sample['frame']] - references[(record['clip'], 17.0)]).mean())
    assert sample['mean_absolute_error'] < wrong_error, (sample, wrong_error)
    negative_controls.append(dict(frame=sample['frame'], correct_error=sample['mean_absolute_error'],
                                  incorrect_1x_error=wrong_error))

# The non-looping jump reaches its endpoint at timeline frame 390 and holds.
hold_error = float(np.abs(decoded[390] - decoded[449]).mean())
assert hold_error < 0.001, hold_error
report = dict(status='passed', probe=probe, samples=samples, negative_controls=negative_controls,
              final_pose_hold_error=hold_error)
(output / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')

captions = []
for index, record in enumerate(records):
    start, end = index * 3, (index + 1) * 3
    mode = 'looping' if record['loop'] else 'one-shot, then hold'
    captions.append(f"{index + 1}\n00:00:{start:02d},000 --> 00:00:{end:02d},000\n"
                    f"{record['clip']} - {record['speed']:g}x - {mode}\n")
subtitle = video.with_suffix('.srt')
subtitle.write_text('\n'.join(captions))
captioned = video.with_name('speed_demo_captioned.mp4')
subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(video), '-i', str(subtitle),
                '-map', '0:v:0', '-map', '1:0', '-c:v', 'copy', '-c:s', 'mov_text',
                '-disposition:s:0', 'default', '-movflags', '+faststart', str(captioned)], check=True)
print('TOONVARA_SPEED_VERIFIED:450 frames; 15 seconds; 35 source-pose image matches; '
      'four wrong-speed controls rejected; final pose held')
print(captioned)
