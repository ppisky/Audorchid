"""CPU-only CLI regression: handoff previews, switches and output protection.

Uses only the Python standard library and generated original test material.
"""
import argparse
import json
import math
import subprocess
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output" / "transition-preview-checks"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/audorchid.exe")
    parser.add_argument("--visual", action="store_true", help="Also verify actual yellow-marker rendering; requires GPU and Pillow")
    options = parser.parse_args()
    binary = options.binary.resolve()
    OUT.mkdir(parents=True, exist_ok=True)

    def run(*args, fail=False):
        p = subprocess.run([str(binary), *map(str, args)], cwd=ROOT, capture_output=True)
        assert (p.returncode != 0) == fail, p.stderr.decode("utf-8", errors="replace")
        return p.stdout

    rate = 48000
    for i in range(3):
        path = OUT / f"source-{i}.wav"
        pcm = bytearray()
        for n in range(rate * 8):
            time = n / rate
            gain = min(1, time / 2) * min(1, (8 - time) / (4 if i == 0 else 2))
            value = round(9000 * gain * math.sin(2 * math.pi * (220 + i * 110) * time))
            pcm.extend(value.to_bytes(2, "little", signed=True) * 2)
        with wave.open(str(path), "wb") as f:
            f.setparams((2, 2, rate, 0, "NONE", "not compressed"))
            f.writeframes(pcm)
    tracks = "\n".join(f'[[tracks]]\naudio = "source-{i}.wav"\n' for i in range(3))
    common = '[audio.arrangement]\nenabled = false\n'
    config = OUT / "smart.toml"
    config.write_text(common + tracks, encoding="utf-8")
    plan = json.loads(run("inspect", config))
    assert plan["arrangement"]["order"] == [0, 1, 2]
    assert plan["arrangement"]["mode"] == "configuration"
    assert any(plan["timeline"]["overlaps"])
    run("audio-preview", config, "--duration", "100", "-o", OUT / "full.wav", "--overwrite")
    with wave.open(str(OUT / "full.wav")) as f:
        full = f.readframes(f.getnframes())

    def compare(number, before, after, path):
        span = plan["timeline"]["tracks"][number]
        overlap = plan["timeline"]["overlaps"][number - 1]
        start = max(0, span["start"] - round(before * rate))
        end = min(plan["timeline"]["total_samples"], span["start"] + overlap + round(after * rate))
        with wave.open(str(path)) as f:
            assert (f.getnchannels(), f.getsampwidth(), f.getframerate()) == (2, 2, rate)
            assert f.getnframes() == end - start
            assert f.readframes(f.getnframes()) == full[start * 4:end * 4]

    # Derived path follows the configured output, not the terminal or source name.
    run("transition-preview", config, "--overwrite")
    derived = OUT / "audorchid.transition-01.wav"
    compare(1, 6, 6, derived)
    run("transition-preview", config, fail=True)
    excerpt = OUT / "second.wav"
    run("transition-preview", config, "--transition", "2", "--before", "0.25", "--after", "0.5",
        "-o", excerpt, "--overwrite")
    compare(2, .25, .5, excerpt)
    for args in [("--transition", "0"), ("--transition", "3"),
                 ("--before", "NaN"), ("--after", "-1"), ("--after", "inf")]:
        run("transition-preview", config, *args, "-o", OUT / "invalid.wav", "--overwrite", fail=True)
    run("transition-preview", config, "-o", OUT / "source-0.wav", "--overwrite", fail=True)
    run("transition-preview", config, "-o", OUT / "invalid.mp4", fail=True)

    # Switch takes precedence over Smart and over a nonzero configured duration.
    disabled = OUT / "disabled.toml"
    disabled.write_text(common + '[audio.transitions]\nenabled = false\n' + tracks, encoding="utf-8")
    plain = json.loads(run("inspect", disabled))
    assert plain["timeline"]["overlaps"] == [0, 0]
    assert plain["timeline"]["total_samples"] == 24 * rate
    assert all(t["overlap_samples"] == 0 for t in plain["transitions"])
    run("transition-preview", disabled, "--before", "0.25", "--after", "0.5", "-o", OUT / "join.wav", "--overwrite")
    with wave.open(str(OUT / "join.wav")) as f:
        assert f.getnframes() == round(.75 * rate)
    run("transition-preview", disabled, "--before", "0", "--after", "0", "-o", OUT / "empty.wav", fail=True)
    report = {
        "default_handoff_and_derived_path": "passed",
        "one_based_second_handoff_with_sample_exact_context": "passed",
        "invalid_selection_context_and_input_overwrite": "passed",
        "arrangement_disabled_uses_configuration_order": "passed",
        "transitions_disabled_has_zero_overlaps_and_direct_join_preview": "passed",
        "wav_excerpt_exactly_matches_final_pcm": "passed",
    }
    if options.visual:
        from PIL import Image, ImageChops

        for base, expected in [(config, True), (disabled, False)]:
            images = []
            for show in [True, False]:
                visual = OUT / f"visual-{base.stem}-{show}.toml"
                source = base.read_text(encoding="utf-8")
                source = source.replace('[[tracks]]', '[visual.progress_style]\nshow_overlap = ' + str(show).lower() + '\n[[tracks]]', 1)
                visual.write_text('[video]\nwidth = 320\nheight = 180\n' + source, encoding="utf-8")
                png = visual.with_suffix(".png")
                run("preview", visual, "--seconds", "3", "-o", png, "--overwrite")
                images.append(Image.open(png).convert("RGB"))
            has_marker = ImageChops.difference(*images).getbbox() is not None
            assert has_marker == expected, (base.stem, has_marker)
        report["actual_gpu_yellow_marker_follows_transition_switch"] = "passed"
    (OUT / "verification.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
