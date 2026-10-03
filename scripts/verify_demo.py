"""Validate generated demo WAVs and MIDI with independent standard-library readers."""
import array
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/release/audorchid.exe'

def midi_info(data):
    assert data[:4] == b'MThd'
    length, fmt, count, ppq = struct.unpack('>IHHH', data[4:14])
    assert (length, fmt, count, ppq) == (6, 1, 5, 480)
    position, note_count, pitches, channels, ends = 14, 0, set(), set(), []
    tempo = None
    for _ in range(count):
        assert data[position:position+4] == b'MTrk'
        size = struct.unpack('>I', data[position+4:position+8])[0]
        track = data[position+8:position+8+size]
        position += 8 + size
        offset, tick, active = 0, 0, set()
        def vlq():
            nonlocal offset
            value = 0
            for _ in range(4):
                byte = track[offset]; offset += 1
                value = (value << 7) | (byte & 127)
                if not byte & 128: return value
            raise AssertionError('Invalid MIDI delta time')
        while offset < len(track):
            tick += vlq()
            status = track[offset]; offset += 1
            if status == 0xff:
                kind = track[offset]; offset += 1
                size = vlq()
                payload = track[offset:offset+size]; offset += size
                if kind == 0x51: tempo = int.from_bytes(payload, 'big')
                if kind == 0x2f: assert not active
            else:
                assert 0x80 <= status < 0xf0
                size = 1 if status >> 4 in (0xc, 0xd) else 2
                payload = track[offset:offset+size]; offset += size
                assert all(value < 128 for value in payload)
                key = (status & 15, payload[0])
                if status >> 4 == 9 and payload[1]:
                    assert key not in active
                    active.add(key); note_count += 1
                    pitches.add(payload[0]); channels.add(status & 15)
                elif status >> 4 == 8 or status >> 4 == 9:
                    assert key in active
                    active.remove(key)
        assert tick == 32 * ppq and not active
        ends.append(tick)
    assert position == len(data) and tempo == 500000
    assert channels == {0, 1, 2, 9} and note_count > 200 and len(pitches) > 15
    return {'format': fmt, 'tracks': count, 'ppq': ppq, 'bpm': 60_000_000/tempo,
            'note_events': note_count, 'distinct_pitches': len(pitches), 'end_ticks': ends}

def main():
    report = []
    with tempfile.TemporaryDirectory(prefix='audorchid-music-') as directory:
        folder = Path(directory)
        for name in ['a', 'b']:
            subprocess.run([BINARY, 'demo', '--dir', folder/name], check=True, capture_output=True)
        for name, sr in [('first',44100), ('second',48000)]:
            pcm_path = folder/'a'/f'{name}.wav'
            assert pcm_path.read_bytes() == (folder/'b'/f'{name}.wav').read_bytes()
            midi = (folder/'a'/f'{name}.mid').read_bytes()
            assert midi == (folder/'b'/f'{name}.mid').read_bytes()
            with wave.open(str(pcm_path),'rb') as stream:
                assert (stream.getnchannels(), stream.getsampwidth(), stream.getframerate(), stream.getnframes()) == (2,2,sr,sr*16)
                pcm = array.array('h', stream.readframes(stream.getnframes()))
            peak = max(abs(x) for x in pcm)/32768
            rms = math.sqrt(sum((x/32768)**2 for x in pcm)/len(pcm))
            dc = sum(pcm)/len(pcm)/32768
            assert .79 < peak < .81 and .03 < rms < .4 and abs(dc) < .01
            assert pcm[::2] != pcm[1::2] and max(abs(x) for x in pcm[-128:]) < 500
            report.append({'name': name, 'sample_rate': sr, 'seconds':16, 'peak':peak,
                           'rms':rms,'dc':dc,'sha256':hashlib.sha256(pcm_path.read_bytes()).hexdigest(),
                           'midi': midi_info(midi), 'deterministic':True})
            print(f'PASS {name}: 16 s stereo, peak {peak:.3f}, RMS {rms:.3f}, valid MIDI, deterministic')
    (ROOT/'docs/research/demo-music-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')

if __name__ == '__main__': main()
