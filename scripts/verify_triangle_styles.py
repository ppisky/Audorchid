"""Actual GPU acceptance for optional triangle presets and opacity/thickness."""
import json
from pathlib import Path
import subprocess
import tempfile
from PIL import Image, ImageChops

ROOT=Path(__file__).resolve().parents[1]
BINARY=ROOT/'target/release/audorchid.exe'

def main():
    with tempfile.TemporaryDirectory(prefix='audorchid-triangles-') as directory:
        folder=Path(directory)
        config=folder/'scene.toml'
        source=f'''schema_version = 1
[output]
path = "test.mp4"
[video]
width = 480
height = 270
[visual]
spectrum = false
waveform = false
[fonts]
latin = "{ROOT.as_posix()}/assets/fonts/Roboto-Regular.ttf"
[[tracks]]
audio = "{ROOT.as_posix()}/examples/media/first.wav"
title = "Triangle acceptance"
trim_end_seconds = 1.0
'''
        def frame(name, count=None, style=''):
            text=source
            if count is not None: text=text.replace('[visual]',f'[visual]\ntriangles = {count}')
            if style: text+='\n[visual.triangle_style]\n'+style+'\n'
            config.write_text(text,encoding='utf-8')
            output=folder/f'{name}.png'
            result=subprocess.run([BINARY,'frame',config,'--seconds','0.2','--output',output],capture_output=True)
            assert result.returncode==0,result.stderr.decode(errors='replace')
            return Image.open(output).convert('RGB')
        reference=frame('default')
        for name,count,style in [
            ('explicit-off',0,''), ('style-only',None,'preset = "border"'),
            ('transparent',48,'opacity = 0.0'),
            ('zero-border',48,'preset = "border"\nthickness = 0.0')]:
            assert frame(name,count,style).tobytes()==reference.tobytes(),name
        counts={}
        border='preset = "border"\nsize = 200.0\ncolor = "#E4E8F8"\ncolor_light = "#B9DCEC"\nopacity = 0.5'
        for name,style in [('layered','preset = "layered"'),('border',border),('filled-border',border+'\nthickness = 1.0')]:
            actual=frame(name,48,style)
            diff=ImageChops.difference(actual,reference)
            counts[name]=sum(any(pixel) for pixel in diff.getdata())
            assert counts[name]>100,name
            if name=='border': assert frame('border-repeat',48,style).tobytes()==actual.tobytes()
        assert counts['filled-border']>counts['border']*1.5,counts
        report={'default_off':True,'style_does_not_enable':True,'opacity_zero_invisible':True,
                'thickness_zero_invisible':True,'repeatable':True,'changed_pixels':counts}
        (ROOT/'docs/research/triangle-styles-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
        print('PASS triangle defaults, presets, zero opacity, 0/1 thickness and deterministic frames:',counts)

if __name__=='__main__': main()
