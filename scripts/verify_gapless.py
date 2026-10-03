"""Compare one continuous WAV with two WAVs split at an arbitrary sample.

Both exports must produce identical decoded AAC, including around the join.
This verifies zero inserted gaps/duplicate samples for lossless equal-rate
input; it does not claim silence detection or arbitrary lossy-file trimming.
"""
import argparse
import array
import hashlib
import json
import math
from pathlib import Path
import subprocess
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[1]

def run(args):
    p = subprocess.run([str(x) for x in args], cwd=ROOT, capture_output=True)
    if p.returncode:
        raise RuntimeError(p.stderr.decode('utf-8', errors='replace'))
    return p.stdout

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/audorchid.exe')
    binary = parser.parse_args().binary.resolve()
    total, split, sr = 96017, 48013, 48000
    pcm = array.array('h')
    for i in range(total):
        sample = int(7000*math.sin(math.tau*437*i/sr) + 2000*math.sin(math.tau*1193*i/sr))
        pcm.extend([sample, sample])
    with tempfile.TemporaryDirectory(prefix='audorchid-gapless-') as temp:
        folder = Path(temp)
        for name, a, b in [('whole',0,total), ('first',0,split), ('second',split,total)]:
            with wave.open(str(folder/f'{name}.wav'),'wb') as w:
                w.setparams((2,2,sr,0,'NONE','not compressed'))
                w.writeframes(pcm[a*2:b*2].tobytes())
        results = {}
        for mode, names in [('whole',['whole']),('split',['first','second'])]:
            output=folder/f'{mode}.mp4'
            config=folder/f'{mode}.toml'
            source=f'''schema_version = 1
[output]
path = "{output.as_posix()}"
[video]
width = 320
height = 180
fps_num = 10
[audio]
sample_rate = 48000
crossfade_seconds = 0.0
[audio.arrangement]
mode = "configuration"
[audio.loudness]
mode = "off"
[visual]
triangles = 0
spectrum = false
[fonts]
latin = "{ROOT.as_posix()}/assets/fonts/Roboto-Regular.ttf"
'''
            for name in names:
                source+=f'\n[[tracks]]\naudio = "{name}.wav"\ntitle = "Gapless verification"\n'
            config.write_text(source,encoding='utf-8')
            plan=json.loads(run([binary,'inspect',config]))
            assert plan['timeline']['total_samples']==total
            if mode=='split': assert plan['chapter_starts']==[0,split]
            run([binary,'render',config])
            probe=json.loads(run(['ffprobe','-v','error','-select_streams','a','-show_streams','-of','json',output]))['streams'][0]
            assert abs(float(probe['duration'])-total/sr)<1/sr
            results[mode]=run(['ffmpeg','-v','error','-i',output,'-map','0:a:0','-f','f32le','-'])
        assert results['whole']==results['split'], 'AAC differs between contiguous and split sources'
        report={'sample_rate':sr,'valid_samples':total,'join_sample':split,'join_seconds':split/sr,
                'decoded_aac_identical':True,'decoded_pcm_sha256':hashlib.sha256(results['whole']).hexdigest(),
                'scope':'lossless stereo WAV, identical sample rates, crossfade=0, loudness=off; decoder tail padding retained equally'}
        (ROOT/'docs/research/gapless-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
        print(f'PASS: {total} valid samples; split at {split}; decoded AAC is byte-identical to one continuous source')

if __name__=='__main__': main()
