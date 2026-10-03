"""Revision acceptance: actual GPU silence, seeking, LUFS and peak protection.

Requires the release executable, Python Pillow, ffmpeg. No network or input music.
"""
import argparse
import array
import json
import math
from pathlib import Path
import re
import subprocess
import tempfile
import wave
from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]

def run(args):
    p = subprocess.run([str(x) for x in args], cwd=ROOT, capture_output=True)
    if p.returncode:
        raise RuntimeError(p.stderr.decode('utf-8', errors='replace'))
    return p.stdout

def write_wave(path, signal):
    values = array.array('h', (round(max(-1., min(1., x)) * 32767) for pair in signal for x in pair))
    with wave.open(str(path), 'wb') as w:
        w.setparams((2, 2, 48000, 0, 'NONE', 'not compressed'))
        w.writeframes(values.tobytes())

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/audorchid.exe')
    args = parser.parse_args()
    binary = args.binary.resolve()
    report = {}
    with tempfile.TemporaryDirectory(prefix='audorchid-revision-') as temp:
        folder = Path(temp)
        write_wave(folder/'tone.wav', ((0.1*math.sin(math.tau*1000*i/48000),)*2 for i in range(48000*3)))
        write_wave(folder/'silence.wav', ((0., 0.) for _ in range(48000)))
        write_wave(folder/'pulse.wav', ((0.8, 0.8) if i % 24000 == 1 else (0., 0.) for i in range(48000*3)))
        write_wave(folder/'gapped.wav', ((0.1*math.sin(math.tau*440*i/48000), -0.1*math.sin(math.tau*440*i/48000)) if 24000 <= i < 120000 else (0.,0.) for i in range(48000*4)))
        base = f'''schema_version = 1
[output]
path = "{folder.as_posix()}/test.mp4"
[video]
width = 360
height = 480
fps_num = 30
[layout]
preset = "3:4"
progress_position = "attached"
[audio]
crossfade_seconds = 0.0
peak_ceiling_db = -1.0
[audio.loudness]
mode = "program"
target_lufs = -16.0
[visual]
waveform = false
spectrum = false
triangles = 0
[fonts]
latin = "{ROOT.as_posix()}/assets/fonts/Roboto-Regular.ttf"
[[tracks]]
audio = "tone.wav"
title = "Audio and visual acceptance"
'''
        config = folder/'test.toml'
        def inspect(text):
            config.write_text(text, encoding='utf-8')
            return json.loads(run([binary, 'inspect', config]))
        plan = inspect(base)
        assert abs(plan['loudness']['output']['integrated_lufs'] + 16.) < 0.03
        assert plan['loudness']['output']['true_peak_dbtp'] <= -1.0 + 0.001
        run([binary, 'render', config])
        analysis = subprocess.run(['ffmpeg','-hide_banner','-i',str(folder/'test.mp4'),'-af','ebur128=peak=true','-f','null','-'],capture_output=True)
        assert analysis.returncode == 0
        summary = analysis.stderr.decode(errors='replace').rsplit('Summary:',1)[-1]
        integrated = float(re.search(r'I:\s*([-\d.]+) LUFS',summary)[1])
        peak = float(re.search(r'Peak:\s*([-\d.]+) dBFS',summary)[1])
        assert abs(integrated + 16.) <= 0.2, summary
        report['loudness'] = {'pre_aac':plan['loudness'], 'ffmpeg_decoded_aac_lufs':integrated,'ffmpeg_decoded_aac_true_peak':peak}
        print(f'PASS programme LUFS: native -16.00, FFmpeg decoded AAC {integrated:.1f}',flush=True)
        limited = inspect(base.replace('tone.wav','pulse.wav').replace('target_lufs = -16.0','target_lufs = -5.0'))['loudness']
        assert limited['peak_attenuation_db'] < 0
        assert limited['output']['true_peak_dbtp'] <= -1.0 + 0.01
        assert limited['output']['integrated_lufs'] < -5.
        report['peak_limited'] = limited
        silent = inspect(base.replace('tone.wav','silence.wav'))['loudness']
        assert silent['output']['integrated_lufs'] is None and silent['normalization_gain_db'] == 0
        print('PASS peak priority and unmeasurable silence',flush=True)
        # Each mode compared against the actual same-time background, never a
        # synthetic expected picture. Both initial and trailing silence tested.
        base = base.replace('tone.wav','gapped.wav')
        reference = {}
        g = inspect(base)['layout']['waveform']
        crop = tuple(map(round,[g[0],g[1],g[0]+g[2],g[1]+g[3]]))
        def frame(text,name,seconds):
            config.write_text(text,encoding='utf-8')
            path=folder/f'{name}.png'
            run([binary,'frame',config,'--seconds',seconds,'--output',path])
            return Image.open(path).convert('RGB').crop(crop)
        for seconds in [0.3,1.5,3.5]: reference[seconds] = frame(base,f'blank-{seconds}',seconds)
        modes = {
            'scope': base.replace('waveform = false','waveform = true'),
            'envelope': base.replace('waveform = false','waveform = true')+'\n[visual.waveform_style]\nmode = "envelope"\n',
            'custom_color': base.replace('spectrum = false','spectrum = true')+'\n[visual.spectrum_style]\ncolor = "#B9DCEC"\npeak_caps = true\n',
            'white': base.replace('spectrum = false','spectrum = true'),
        }
        for mode,text in modes.items():
            for seconds in [0.3,1.5,3.5]:
                actual=frame(text,f'{mode}-{seconds}',seconds)
                different=ImageChops.difference(actual,reference[seconds]).getbbox() is not None
                assert different == (seconds == 1.5),(mode,seconds)
            repeated=frame(text,f'{mode}-repeat',1.5)
            first=Image.open(folder/f'{mode}-1.5.png').convert('RGB').crop(crop)
            assert repeated.tobytes()==first.tobytes()
            print(f'PASS {mode}: silent regions identical to background, audible region visible, deterministic seek',flush=True)
        report['visual_modes'] = list(modes)
        report['silence_hidden_before_and_after'] = True
        report['deterministic_seek'] = True
    (ROOT/'docs/research/visual-audio-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')

if __name__ == '__main__': main()
