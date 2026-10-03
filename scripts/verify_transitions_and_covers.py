"""End-to-end checks using original synthetic material, never private assets."""
import json
import math
import subprocess
import wave
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output" / "transitions-checks"
BINARY = ROOT / "target" / "release" / "audorchid.exe"


def run(*args, fail=False):
    result = subprocess.run([str(BINARY), *map(str, args)], capture_output=True)
    assert (result.returncode != 0) == fail, result.stderr.decode("utf-8", errors="replace")
    return result.stdout


def source(path, fade):
    rate = 48000
    pcm = bytearray()
    for i in range(rate * 8):
        t = i / rate
        gain = min(1.0, t / 2.0) * min(1.0, (8.0 - t) / fade)
        left = round(10000 * gain * math.sin(2 * math.pi * 220 * t))
        right = round(10000 * gain * math.sin(2 * math.pi * 330 * t))
        pcm += left.to_bytes(2, "little", signed=True) + right.to_bytes(2, "little", signed=True)
    with wave.open(str(path), "wb") as f:
        f.setnchannels(2)
        f.setsampwidth(2)
        f.setframerate(rate)
        f.writeframes(pcm)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for i in range(3):
        source(OUT / f"source-{i}.wav", 4 if i == 0 else 2)
    tracks = "\n".join(f'[[tracks]]\naudio = "source-{i}.wav"\n' for i in range(3))
    fixed = OUT / "fixed.toml"
    fixed.write_text('[audio.transitions]\nmode = "fixed"\n[audio.arrangement]\nmode = "configuration"\n' + tracks, encoding="utf-8")
    adaptive = OUT / "adaptive.toml"
    adaptive.write_text('[audio.transitions]\nmode = "adaptive"\n[audio.arrangement]\nmode = "configuration"\n' + tracks, encoding="utf-8")
    fixed_plan = json.loads(run("inspect", fixed))
    plan = json.loads(run("inspect", adaptive))
    assert fixed_plan["timeline"]["overlaps"] == [96000, 96000]
    assert plan["arrangement"]["order"] == [0, 1, 2]
    assert all(t["mode"] == "adaptive" for t in plan["transitions"])
    assert plan["transitions"][0]["estimated_dip_db"] < fixed_plan["transitions"][0]["estimated_dip_db"]
    spans = plan["timeline"]["tracks"]
    overlaps = plan["timeline"]["overlaps"]
    assert sum(t["length"] for t in spans) - sum(overlaps) == plan["timeline"]["total_samples"]
    assert overlaps[0] + overlaps[1] <= spans[1]["length"]
    full, excerpt = OUT / "full.wav", OUT / "excerpt.wav"
    run("audio-preview", adaptive, "--duration", "100", "-o", full, "--overwrite")
    run("audio-preview", adaptive, "--seconds", ".25", "--duration", ".5", "-o", excerpt, "--overwrite")
    with wave.open(str(full)) as f, wave.open(str(excerpt)) as e:
        assert f.getnframes() == plan["timeline"]["total_samples"]
        assert (e.getframerate(), e.getnchannels(), e.getsampwidth(), e.getnframes()) == (48000, 2, 2, 24000)
        f.setpos(12000)
        assert f.readframes(24000) == e.readframes(24000)
    run("audio-preview", adaptive, "-o", full, fail=True)
    run("audio-preview", adaptive, "-o", OUT / "source-0.wav", "--overwrite", fail=True)
    run("audio-preview", adaptive, "--seconds", "999", "-o", OUT / "invalid.wav", fail=True)
    run("audio-preview", adaptive, "--duration", "NaN", "-o", OUT / "invalid.wav", fail=True)
    a, b, c = [OUT / name for name in ["cover-a.png", "cover-b.png", "cover-c.png"]]
    for path, seed in [(a, 42), (b, 42), (c, 43)]:
        run("cover", "-o", path, "--size", "256", "--seed", seed, "--overwrite")
    assert a.read_bytes() == b.read_bytes() and a.read_bytes() != c.read_bytes()
    with Image.open(a) as image:
        assert image.size == (256, 256) and image.format == "PNG"
    run("cover", "-o", a, fail=True)
    run("cover", "-o", OUT / "invalid.png", "--size", "0", fail=True)
    disguised = OUT / "png-with-jpg-name.jpg"
    disguised.write_bytes(a.read_bytes())
    image_config = OUT / "images.toml"
    image_config.write_text('[video]\nwidth = 320\nheight = 180\n[[tracks]]\naudio = "source-0.wav"\ncover = "png-with-jpg-name.jpg"\n', encoding="utf-8")
    run("preview", image_config, "--seconds", "3", "-o", OUT / "disguised-image.png", "--overwrite")
    generated_config = OUT / "generated.toml"
    generated_config.write_text('[video]\nwidth = 320\nheight = 180\n[[tracks]]\naudio = "source-0.wav"\ncover_seed = 42\n', encoding="utf-8")
    run("preview", generated_config, "--seconds", "3", "-o", OUT / "generated-image.png", "--overwrite")
    run("render", adaptive, "--scale", ".1666666667", "-o", OUT / "adaptive.mp4", "--overwrite")
    subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(OUT / "adaptive.mp4"), "-f", "null", "-"], check=True)
    report = {"adaptive_energy_and_variable_timeline": "passed", "audio_excerpt_sample_alignment": "passed", "input_overwrite_and_invalid_ranges": "passed", "deterministic_cover_and_bounds": "passed", "png_content_with_jpg_extension": "passed", "generated_cover_seed_in_toml": "passed", "variable_overlap_video_full_decode": "passed"}
    (OUT / "verification.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
