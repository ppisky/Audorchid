"""Real GPU checks of minimal config, embedded fonts and adaptive spectrum density."""
import array
import json
from pathlib import Path
import random
import subprocess
import tempfile
import wave

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/release/audorchid.exe"
OUT = ROOT / "output/defaults-checks"


def run(*args):
    p = subprocess.run([str(x) for x in args], cwd=ROOT, capture_output=True)
    if p.returncode:
        raise RuntimeError(p.stderr.decode("utf-8", errors="replace"))
    return p.stdout


def main():
    OUT.mkdir(exist_ok=True)
    full = (ROOT / "examples/audorchid.en.toml").read_text(encoding="utf-8")
    full = full.replace("../assets/", (ROOT / "assets").as_posix() + "/")
    full = full.replace('"media/', '"' + (ROOT / "examples/media").as_posix() + "/")
    simple = (ROOT / "examples/audorchid.simple.en.toml").read_text(encoding="utf-8")
    simple = simple.replace('"media/', '"' + (ROOT / "examples/media").as_posix() + "/")
    report = {"presets": []}
    with tempfile.TemporaryDirectory(prefix="audorchid-defaults-") as temp:
        folder = Path(temp)

        def config(name, source):
            path = folder / f"{name}.toml"
            path.write_text(source, encoding="utf-8")
            return path

        def frame(name, source, seconds=4.2):
            path = config(name, source)
            destination = OUT / f"{name}.png"
            run(BINARY, "preview", path, "--seconds", seconds, "--scale", "0.5", "-o", destination, "--overwrite")
            return Image.open(destination).convert("RGB"), json.loads(run(BINARY, "inspect", path))

        full_image, full_plan = frame("complete", full)
        simple_image, simple_plan = frame("simple", simple)
        # Smart changes overlap markers and may change programme peak gain, so
        # compare the explicit fixed demo with the same explicitly fixed mode.
        fixed_image, fixed_plan = frame("simple-fixed", simple.replace('[audio.transitions]', '[audio.transitions]\nmode = "fixed"', 1))
        assert ImageChops.difference(full_image, fixed_image).getbbox() is None
        for field in ["timeline", "audio_settings", "loudness"]:
            assert full_plan[field] == fixed_plan[field], field
        for field in ["layout", "spectrum_settings", "visual_settings", "arrangement"]:
            assert full_plan[field] == simple_plan[field], field
        assert full_plan["audio_settings"]["transitions"]["mode"] == "fixed"
        assert simple_plan["audio_settings"]["transitions"]["mode"] == "smart"
        assert simple_plan["timeline"]["total_samples"] < full_plan["timeline"]["total_samples"]
        assert simple_plan["arrangement"]["mode"] == "automatic"
        assert simple_plan["arrangement"]["order"] == [1, 0]
        ordered = config("ordered", simple.replace('[audio.arrangement]', '[audio.arrangement]\nmode = "configuration"', 1))
        assert json.loads(run(BINARY, "inspect", ordered))["arrangement"]["order"] == [0, 1]
        report["explicit_fixed_simple_complete_rgb_identical"] = True
        report["default_smart_and_explicit_fixed_demo"] = True
        report["automatic_order"] = simple_plan["arrangement"]["order"]
        report["explicit_order"] = [0, 1]

        bare = config("bare", f'[[tracks]]\naudio = "{(ROOT / "examples/media/first.wav").as_posix()}"\n')
        plan = json.loads(run(BINARY, "inspect", bare))
        assert (plan["width"], plan["height"]) == (1920, 1080)
        run(BINARY, "preview", bare, "--scale", "0.5", "-o", OUT / "filename-placeholder.png", "--overwrite")
        report["only_audio_required_and_embedded_fonts"] = True

        # Broad-band noise ensures every frequency bar is visible; count actual
        # separated white stripes, not just a settings value printed by inspect.
        rng = random.Random(7319)
        pcm = array.array("h")
        for _ in range(3 * 48000):
            value = rng.randrange(-16000, 16001)
            pcm.extend([value, value])
        noise = folder / "noise.wav"
        with wave.open(str(noise), "wb") as wav:
            wav.setparams((2, 2, 48000, 0, "NONE", "not compressed"))
            wav.writeframes(pcm.tobytes())
        for ratio, width, height, expected in [("16:9", 1920, 1080, 80), ("9:16", 1080, 1920, 40),
                                                ("1:1", 1080, 1080, 43), ("3:4", 1080, 1440, 43)]:
            prefix = f'[layout]\npreset = "{ratio}"\n'
            image, plan = frame("layout-" + ratio.replace(":", "-"), prefix + simple)
            assert image.size == (width // 2, height // 2)
            assert plan["spectrum_settings"]["bars"] == expected
            spectrum = f'{prefix}[[tracks]]\naudio = "{noise.as_posix()}"\n'
            image, plan = frame("density-" + ratio.replace(":", "-"), spectrum, 0.75)
            x, y, w, h = [round(v / 2) for v in plan["layout"]["waveform"]]
            crop = image.crop((x, y, x + w, y + h))
            occupied = [any(min(crop.getpixel((i, j))) > 220 for j in range(h)) for i in range(w)]
            count = sum(active and (i == 0 or not occupied[i - 1]) for i, active in enumerate(occupied))
            assert count == expected, (ratio, count, expected)
            report["presets"].append({"ratio": ratio, "preview": list(image.size), "bars": expected,
                                      "visible_stripes": count, "reference_width": plan["layout"]["waveform"][2]})
        fixed = config("fixed", '[layout]\npreset = "9:16"\n[visual.spectrum_style]\nbars = 64\n' + simple)
        assert json.loads(run(BINARY, "inspect", fixed))["spectrum_settings"]["bars"] == 64
        report["manual_count_override"] = True
    (ROOT / "docs/research/defaults-validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
