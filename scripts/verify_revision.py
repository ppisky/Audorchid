"""GPU regression checks for typography/layout, transition marks and encoding.

Requires Pillow, ffmpeg and ffprobe. Decoding is an independent test oracle;
Audorchid itself does not invoke FFmpeg. Keeps previews/reports for review.
"""
import json
import math
from pathlib import Path
import subprocess
import tempfile

from PIL import Image, ImageChops, ImageStat

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/release/audorchid.exe'
OUT = ROOT / 'output/revision-checks'


def run(*args):
    result = subprocess.run([str(a) for a in args], cwd=ROOT, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.decode('utf-8', errors='replace'))
    return result.stdout


def base():
    source = (ROOT / 'examples/audorchid.en.toml').read_text(encoding='utf-8')
    return source.replace('../assets/', (ROOT / 'assets').as_posix() + '/').replace(
        '"media/', '"' + (ROOT / 'examples/media').as_posix() + '/')


def main():
    OUT.mkdir(exist_ok=True)
    report = {'layouts': [], 'quality_rgb_psnr_db': {}, 'title_luma_psnr_db': {}}
    with tempfile.TemporaryDirectory(prefix='audorchid-revision-') as folder:
        directory = Path(folder)

        def config(name, source):
            p = directory / f'{name}.toml'
            p.write_text(source.replace('../output/audorchid.mp4',
                                        (directory / f'{name}.mp4').as_posix()), encoding='utf-8')
            return p

        def frame(name, source, seconds=4.2):
            p = config(name, source)
            path = directory / f'{name}.png'
            run(BINARY, 'frame', p, '--seconds', seconds, '--output', path)
            image = Image.open(path).convert('RGB')
            image.save(OUT / f'{name}.png')
            return image, json.loads(run(BINARY, 'inspect', p))

        def yellow(image, plan):
            x, y, width, height = plan['layout']['progress']
            row = int(y + height / 2)
            return [i for i in range(math.ceil(x), math.floor(x+width))
                    if (lambda c: c[0] > c[1]+5 and c[1] > c[2]+10)(image.getpixel((i, row)))]

        for ratio, width, height in [('16:9',1920,1080), ('9:16',1080,1920),
                                     ('1:1',1080,1080), ('3:4',1080,1440)]:
            source = base().replace('preset = "16:9"', f'preset = "{ratio}"')
            source = source.replace('width = 1920 ', f'width = {width} ').replace('height = 1080 ', f'height = {height} ')
            image, plan = frame('layout-'+ratio.replace(':','-'),source)
            g = plan['layout']
            assert g['progress'][0] == g['artist'][0]
            assert g['progress'][2] == g['time'][2]
            assert g['progress'][1] > g['artist'][1]+g['artist'][3]
            marks = yellow(image,plan)
            assert abs(min(marks)-(g['progress'][0]+g['progress'][2]*14/16)) < 2
            assert abs(max(marks)-(g['progress'][0]+g['progress'][2])) < 3
            report['layouts'].append({'ratio':ratio,'size':[width,height],'progress':g['progress']})
            print('PASS layout and outgoing overlap',ratio,flush=True)

        image, plan = frame('handoff',base(),15.)
        marks = yellow(image,plan); x,_,width,_ = plan['layout']['progress']
        assert abs(min(marks)-x) < 2 and abs(max(marks)-(x+width*2/16)) < 3
        image, plan = frame('program',base().replace('scope = "track"','scope = "program"'),15.)
        marks = yellow(image,plan); x,_,width,_ = plan['layout']['progress']
        assert abs(min(marks)-(x+width*14/30)) < 2
        assert abs(max(marks)-(x+width*16/30)) < 3
        image, plan = frame('gapless',base().replace('crossfade_seconds = 2.0','crossfade_seconds = 0.0'))
        assert yellow(image,plan) == []
        scaled = base().replace('# progress_scale = [0.8, 1.0]', 'progress_scale = [0.8, 1.0]')
        counter,plan = frame('counter-scaled',scaled)
        hidden,_ = frame('counter-hidden',scaled.replace('show_counter = true','show_counter = false'))
        difference = ImageChops.difference(counter,hidden).getbbox()
        right = sum(plan['layout']['progress'][i] for i in [0,2])
        assert abs(difference[2]-right) < 2, (difference,right)
        report['counter_right_error_px'] = round(abs(difference[2]-right),3)
        print('PASS per-track/program overlap, gapless and scaled counter alignment',flush=True)

        # Exactly the same scene/PCM/frames in each encoding; no rescaling or
        # transfer-function filter when comparing display-referred RGB values.
        sections = base().split('\n[[tracks]]')
        short = sections[0]+'\n[[tracks]]'+sections[1]
        short = short.replace('crossfade_seconds = 2.0','crossfade_seconds = 0.0').replace(
            'trim_start_seconds = 0.0','trim_start_seconds = 0.0\ntrim_end_seconds = 1.0')
        reference,_ = frame('quality-reference',short,0.5)
        for name,mode,qp in [('quality18','quality',18),('quality38','quality',38),('bitrate','bitrate',18)]:
            source = short.replace('rate_control = "quality"',f'rate_control = "{mode}"').replace('quality_qp = 18',f'quality_qp = {qp}')
            p = config(name,source)
            run(BINARY,'render',p)
            data = run('ffmpeg','-v','error','-i',directory/f'{name}.mp4','-vf','select=eq(n\\,15)',
                       '-frames:v','1','-f','rawvideo','-pix_fmt','rgb24','-')
            decoded = Image.frombytes('RGB',reference.size,data)
            decoded.save(OUT/f'decoded-{name}.png')
            quality = {}
            for region,box in [('title',(870,340,1790,424)),('cover',(150,190,730,770)),('full',(0,0,1920,1080))]:
                a,b = reference.crop(box),decoded.crop(box)
                mse = sum(ImageStat.Stat(ImageChops.difference(a,b)).sum2)/(a.width*a.height*3)
                quality[region] = round(10*math.log10(255**2/mse),3)
            report['quality_rgb_psnr_db'][name] = quality
            a = reference.crop((870,340,1790,424)).convert('L',(.2126,.7152,.0722,0))
            b = decoded.crop((870,340,1790,424)).convert('L',(.2126,.7152,.0722,0))
            mse = ImageStat.Stat(ImageChops.difference(a,b)).sum2[0]/(a.width*a.height)
            report['title_luma_psnr_db'][name] = round(10*math.log10(255**2/mse),3)
            print('Quality',name,quality,flush=True)
        quality = report['quality_rgb_psnr_db']
        # 4:2:0 averages purple background chroma into white glyph edges even
        # before compression. Measure text luminance separately from RGB error.
        assert report['title_luma_psnr_db']['quality18'] > 42
        assert quality['quality18']['title'] > quality['bitrate']['title']+1
        assert quality['quality18']['title'] > quality['quality38']['title']+3
        assert quality['quality18']['full'] > 40
    (ROOT/'docs/research/visual-revision-validation.json').write_text(
        json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('PASS visual and encoding revision',flush=True)


if __name__ == '__main__':
    main()
