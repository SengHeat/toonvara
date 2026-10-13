"""Verify the example MP4 against independent single-clip reference renders.

blender --background --python-exit-code 1 --python adapters/blender/verify_schedule_video.py
Default input: target/schedule-verification/hello_cartoon.mp4
Optional input after --. Writes decoded PNGs, reference PNGs and verification.json.
"""

import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
from unittest.mock import patch

import bpy
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
video = Path(sys.argv[sys.argv.index('--') + 1]) if '--' in sys.argv else ROOT / 'target/schedule-verification/hello_cartoon.mp4'
output = video.parent / 'verification'
output.mkdir(parents=True, exist_ok=True)
project = json.loads((ROOT / 'examples/hello_cartoon.json').read_text())
probe = json.loads(subprocess.check_output([
    'ffprobe', '-v', 'error', '-select_streams', 'v:0', '-count_frames',
    '-show_entries', 'stream=width,height,r_frame_rate,nb_read_frames,duration', '-of', 'json', str(video),
]))['streams'][0]
assert probe['nb_read_frames'] == '450', probe
assert probe['r_frame_rate'] == '30/1', probe
assert abs(float(probe['duration']) - 15) < 0.01, probe
records = [json.loads(line.split(':', 1)[1]) for line in video.with_suffix('.render.log').read_text().splitlines()
           if line.startswith('TOONVARA_SCHEDULE:')]
assert [(r['clip'], r['start'], r['end']) for r in records] == [
    ('walk', 0, 120), ('wave', 120, 180), ('jump', 180, 240), ('idle', 240, 450)]


def pixels(path):
    img = bpy.data.images.load(str(path), check_existing=False)
    data = np.asarray(img.pixels[:], dtype=np.float32).reshape(-1, 4)[:, :3].copy()
    bpy.data.images.remove(img)
    return data


samples = [0, 15, 119, 120, 135, 179, 180, 195, 210, 239, 240, 255, 449]
decoded = {}
for frame in samples:
    path = output / f'decoded_{frame:04d}.png'
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(video), '-vf',
                    f'select=eq(n\\,{frame})', '-frames:v', '1', str(path)], check=True)
    decoded[frame] = pixels(path)

camera = project['scenes'][0]['camera']
references = {}
results = []
for frame in samples:
    record = next(r for r in records if r['start'] <= frame < r['end'])
    native = record['source_start'] + (frame - record['start']) % (record['source_end'] - record['source_start'])
    assert native == int(native), 'This example verification expects integer clip sample times'
    key = (record['clip'], int(native))
    if key not in references:
        prefix = output / f'reference_{key[0]}_{key[1]}_'
        env = {k: v for k, v in os.environ.items() if not k.startswith('TOONVARA_')}
        env.update({
            'TOONVARA_CHARACTER_GLB': str(ROOT / 'examples' / project['assets']['characters']['hero']),
            'TOONVARA_CLIP': key[0], 'TOONVARA_FPS': '30',
            'TOONVARA_FRAME_START': str(key[1]), 'TOONVARA_FRAME_END': str(key[1]),
            'TOONVARA_OUT_PATH': str(prefix),
            'TOONVARA_RES_X': str(probe['width']), 'TOONVARA_RES_Y': str(probe['height']),
        })
        for index, axis in enumerate('XYZ'):
            env[f'TOONVARA_CAM_POS_{axis}'] = str(camera['position'][index])
            env[f'TOONVARA_CAM_TGT_{axis}'] = str(camera['target'][index])
        with patch.dict(os.environ, env, clear=True):
            runpy.run_path(str(Path(__file__).with_name('render.py')), run_name='__main__')
        references[key] = pixels(Path(f'{prefix}{key[1]:04d}.png'))
    error = float(np.abs(decoded[frame] - references[key]).mean())
    assert error < 0.015, (frame, key, error)
    results.append(dict(frame=frame, seconds=frame / 30, clip=key[0], source_frame=key[1], mean_absolute_error=error))

# The four interior frames must each be closest to their own clip, not merely
# close to a common background. This catches a reordered/unchanging MP4.
for frame, expected in zip([15, 135, 195, 255], ['walk', 'wave', 'jump', 'idle']):
    errors = {name: float(np.abs(decoded[frame] - references[(name, 15)]).mean())
              for name in ['walk', 'wave', 'jump', 'idle']}
    assert min(errors, key=errors.get) == expected, (frame, expected, errors)
    print('TOONVARA_VIDEO_MATCH:' + json.dumps(dict(frame=frame, expected=expected, errors=errors)))

report = dict(video=str(video), probe=probe, samples=results, status='passed')
(output / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
print('TOONVARA_VIDEO_VERIFIED:450 frames; 15 seconds; 13 independent reference matches; walk -> wave -> jump -> idle')
