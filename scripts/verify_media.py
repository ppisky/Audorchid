"""End-to-end acceptance checks. Requires a built Audorchid, ffprobe and ffmpeg.

FFmpeg is an independent test decoder, never Audorchid's export backend.
Run: python scripts/verify_media.py --binary target/release/audorchid.exe
"""
import argparse
import array
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib
import wave

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    result = subprocess.run([str(a) for a in args], cwd=ROOT, capture_output=True, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{args}\n{result.stderr.decode('utf-8', errors='replace')}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=Path('target/release/audorchid.exe'))
    opts = parser.parse_args()
    binary = opts.binary.resolve()
    zh = (ROOT / 'examples/audorchid.zh-CN.toml').read_text(encoding='utf-8-sig')
    en = (ROOT / 'examples/audorchid.en.toml').read_text(encoding='utf-8-sig')
    assert tomllib.loads(zh) == tomllib.loads(en), 'Bilingual examples have drifted'
    reports = []
    with tempfile.TemporaryDirectory(prefix='audorchid-acceptance-') as directory:
        folder = Path(directory)
        run([binary, 'demo', '--dir', folder / 'media'])
        for preset, width, height in [('16:9', 640, 360), ('9:16', 360, 640), ('3:4', 360, 480), ('1:1', 360, 360)]:
            output = folder / f'{width}x{height}.mp4'
            config = folder / f'{width}x{height}.toml'
            text = f'''schema_version = 1
[output]
path = "{output.as_posix()}"
[layout]
preset = "{preset}"
[video]
width = {width}
height = {height}
fps_num = 30000
fps_den = 1001
[audio]
sample_rate = 44100
crossfade_seconds = 0.5
[visual]
spectrum = true
[fonts]
latin = "{(ROOT / 'assets/fonts/Roboto-Regular.ttf').as_posix()}"
cjk = "{(ROOT / 'assets/fonts/DreamHanSansCN-W10.ttf').as_posix()}"
[[tracks]]
audio = "media/first.wav"
cover = "media/first.png"
title = "晨光 / Morning light"
trim_end_seconds = 2.0
[[tracks]]
audio = "media/second.wav"
cover = "media/second.png"
title = "月下 / Moonrise"
trim_end_seconds = 2.0
'''
            config.write_text(text, encoding='utf-8')
            run([binary, 'check', config])
            plan = json.loads(run([binary, 'inspect', config]))
            assert plan['timeline']['total_samples'] == 154350
            run([binary, 'render', config])
            # Existing output must not prevent read-only validation.
            run([binary, 'check', config])
            probe = json.loads(run(['ffprobe', '-v', 'error', '-show_streams', '-show_format', '-of', 'json', output]))
            video, audio = probe['streams']
            assert (video['width'], video['height']) == (width, height)
            assert video['codec_name'] == 'h264' and video['pix_fmt'] == 'yuv420p'
            assert video['color_space'] == video['color_transfer'] == video['color_primaries'] == 'bt709'
            assert video['color_range'] == 'tv'
            assert int(video['nb_frames']) == plan['frames']
            assert audio['codec_name'] == 'aac' and audio['channels'] == 2
            assert audio['sample_rate'] == '44100' and audio['time_base'] == '1/44100'
            assert abs(float(audio['duration']) - 3.5) < 1/44100
            assert float(audio['start_time']) == float(video['start_time']) == 0.0
            assert abs(float(video['duration']) - plan['frames']*1001/30000) < 1/90000 + 1e-6
            run(['ffmpeg', '-v', 'error', '-i', output, '-f', 'null', '-'])
            packets = json.loads(run(['ffprobe', '-v', 'error', '-select_streams', 'a', '-show_packets', '-of', 'json', output]))['packets']
            assert any(int(s.get('skip_samples', 0)) == 2048 for s in packets[0].get('side_data_list', [])), 'AAC priming not exposed'
            # Refusing overwrite must leave the existing bytes untouched.
            before = output.read_bytes()
            refused = subprocess.run([str(binary), 'render', str(config)], cwd=ROOT, capture_output=True)
            assert refused.returncode != 0 and output.read_bytes() == before
            reports.append({'preset': preset, 'width': width, 'height': height, 'frames': plan['frames'], 'audio_samples': 154350, 'decode': 'pass'})
            print(f'PASS {preset}: {plan["frames"]} frames, 3.5 s AAC, clean decode', flush=True)
        # Actual GPU preview of a fully custom layout, including reproducible seek.
        custom = text.replace('preset = "1:1"', 'preset = "custom"') + '''
[layout.custom]
cover = [30.0, 25.0, 120.0, 120.0]
title = [30.0, 160.0, 300.0, 50.0]
artist = [30.0, 214.0, 300.0, 20.0]
waveform = [30.0, 245.0, 300.0, 55.0]
progress = [30.0, 320.0, 300.0, 3.0]
time = [30.0, 335.0, 300.0, 20.0]
title_size = 20.0
artist_size = 12.0
time_size = 10.0
'''
        config.write_text(custom, encoding='utf-8')
        for name, seconds in [('a', 0.25), ('b', 2.5), ('repeat', 0.25)]:
            run([binary, 'frame', config, '--seconds', seconds, '--output', folder / f'{name}.png'])
        assert (folder/'a.png').read_bytes() == (folder/'repeat.png').read_bytes()
        assert (folder/'a.png').read_bytes() != (folder/'b.png').read_bytes()
        print('PASS custom layout and deterministic repeated frame', flush=True)

        # Impulses close to both ends check actual AAC alignment and tail draining.
        valid_samples = 96017
        impulses = [4800, valid_samples-100]
        pcm = array.array('h', [0]) * (valid_samples*2)
        for index in impulses:
            pcm[index*2] = pcm[index*2+1] = 24000
        with wave.open(str(folder/'media/impulse.wav'), 'wb') as audio_file:
            audio_file.setparams((2, 2, 48000, 0, 'NONE', 'not compressed'))
            audio_file.writeframes(pcm.tobytes())
        output = folder/'impulse.mp4'
        impulse_config = text.split('[[tracks]]')[0].replace('sample_rate = 44100', 'sample_rate = 48000')
        impulse_config = impulse_config.replace(f'{folder.as_posix()}/360x360.mp4', output.as_posix())
        impulse_config += '\n[[tracks]]\naudio = "media/impulse.wav"\ntitle = "Impulse"\n'
        config.write_text(impulse_config, encoding='utf-8')
        run([binary, 'render', config])
        decoded = array.array('f')
        decoded.frombytes(run(['ffmpeg', '-v', 'error', '-i', output, '-map', '0:a:0', '-f', 'f32le', '-']))
        offsets = []
        for expected in impulses:
            peak = max(range(expected-12, expected+13), key=lambda i: abs(decoded[i*2]))
            assert abs(peak-expected) <= 2 and abs(decoded[peak*2]) > 0.1
            offsets.append(peak-expected)
        print(f'PASS AAC impulse alignment: sample offsets {offsets}; tail impulse retained', flush=True)
        impulse_report = {'valid_samples': valid_samples, 'decoder_samples_including_final_frame_padding': len(decoded)//2, 'impulse_offsets_samples': offsets}
    report = ROOT / 'docs/research/media-validation.json'
    report.write_text(json.dumps({'bilingual_equivalent': True, 'cases': reports, 'custom_and_deterministic_preview': True, 'aac_impulses': impulse_report}, indent=2)+'\n', encoding='utf-8')


if __name__ == '__main__':
    main()
