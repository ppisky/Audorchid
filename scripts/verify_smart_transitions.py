"""Exercise default automix with original test tones, not private recordings."""
import json
import subprocess
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "output" / "smart-checks"
BINARY = ROOT / "target" / "release" / "audorchid.exe"
RATE = 48000


def run(*args, fail=False):
    p = subprocess.run([str(BINARY), *map(str, args)], capture_output=True)
    assert (p.returncode != 0) == fail, p.stderr.decode("utf-8", errors="replace")
    return p.stdout


def wav(path):
    with wave.open(str(path)) as f:
        assert (f.getframerate(), f.getnchannels(), f.getsampwidth()) == (RATE, 2, 2)
        return np.frombuffer(f.readframes(f.getnframes()), dtype="<i2").reshape(-1, 2)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    time = np.arange(32 * RATE) / RATE
    for i, (intro, quiet, fade) in enumerate([(0, 5, 4), (1, 1, 3), (2, 0, 2)]):
        gain = np.clip((time - intro) / 1.2, 0, 1) * np.clip((32 - quiet - time) / fade, 0, 1)
        left = 0.15 * gain * np.sin(2 * np.pi * (263 + 31 * i) * time)
        right = 0.15 * gain * np.sin(2 * np.pi * (397 + 53 * i) * time)
        pcm = np.rint(np.stack([left, right], axis=1) * 32767).astype("<i2")
        with wave.open(str(OUT / f"source-{i}.wav"), "wb") as f:
            f.setparams((2, 2, RATE, 0, "NONE", "not compressed"))
            f.writeframes(pcm.tobytes())
    tracks = "\n".join(f'[[tracks]]\naudio = "source-{i}.wav"\n' for i in range(3))
    common = '[video]\nwidth = 320\nheight = 180\nfps_num = 15\n[audio.arrangement]\nmode = "configuration"\n'
    smart = OUT / "smart.toml"
    smart.write_text(common + tracks, encoding="utf-8")
    fixed = OUT / "fixed.toml"
    fixed.write_text(common + '[audio.transitions]\nmode = "fixed"\n' + tracks, encoding="utf-8")
    plan = json.loads(run("inspect", smart))
    assert plan["audio_settings"]["transitions"]["mode"] == "smart"
    assert plan["arrangement"]["order"] == [0, 1, 2]
    assert all(t["mode"] == "smart" for t in plan["transitions"])
    first = plan["transitions"][0]
    assert first["quiet_tail_seconds"] > 4.5
    assert first["fade_tail_seconds"] > first["quiet_tail_seconds"]
    assert first["intro_seconds"] > 0.7
    assert first["overlap_seconds"] > first["quiet_tail_seconds"] + first["intro_seconds"]
    assert first["outgoing_local_lufs"] is not None and first["incoming_local_lufs"] is not None
    assert abs(first["incoming_match_db"]) <= 3
    spans = plan["timeline"]["tracks"]
    overlaps = plan["timeline"]["overlaps"]
    assert overlaps[0] + overlaps[1] <= spans[1]["length"]
    assert sum(s["length"] for s in spans) - sum(overlaps) == plan["timeline"]["total_samples"]
    assert plan["loudness"]["output"]["true_peak_dbtp"] <= -1 + 1e-6
    run("audio-preview", smart, "--duration", "100", "-o", OUT / "full.wav", "--overwrite")
    full = wav(OUT / "full.wav")
    assert len(full) == plan["timeline"]["total_samples"]
    start = spans[1]["start"]
    run("audio-preview", smart, "--seconds", str(start / RATE), "--duration", "1",
        "-o", OUT / "excerpt.wav", "--overwrite")
    assert np.array_equal(full[start:start + RATE], wav(OUT / "excerpt.wav"))
    # Constant-tone material exposes a hole: the old two-second transition sits
    # completely inside the outgoing five-second silent tail.
    fixed_plan = json.loads(run("inspect", fixed))
    fixed_start = fixed_plan["timeline"]["tracks"][1]["start"]
    run("audio-preview", fixed, "--seconds", str(fixed_start / RATE), "--duration", "2",
        "-o", OUT / "fixed-hole.wav", "--overwrite")
    def minimum_rms(pcm):
        blocks = len(pcm) // (RATE // 2)
        return min(10 * np.log10(np.mean(b.astype(float) ** 2) / 32768 ** 2 + 1e-15)
                   for b in pcm[:blocks * (RATE // 2)].reshape(blocks, RATE // 2, 2))
    minimum = minimum_rms(full[start:start + overlaps[0]])
    old_minimum = minimum_rms(wav(OUT / "fixed-hole.wav"))
    assert minimum > old_minimum + 20, (minimum, old_minimum)
    dry = OUT / "dry.toml"
    dry.write_text(common + '[audio.transitions]\ntone_strength = 0\nreverb_mix = 0\n' + tracks, encoding="utf-8")
    run("audio-preview", dry, "--duration", "100", "-o", OUT / "dry.wav", "--overwrite")
    dry_pcm = wav(OUT / "dry.wav")
    assert len(dry_pcm) == len(full)
    assert np.mean((full[start:start + overlaps[0]].astype(float) -
                    dry_pcm[start:start + overlaps[0]].astype(float)) ** 2) > 1
    invalid = OUT / "invalid.toml"
    for bad in ['reverb_mix = 0.8', 'tail_threshold_db = -2', 'max_gain_db = 99', 'analysis_seconds = 2']:
        invalid.write_text(common + '[audio.transitions]\n' + bad + '\n' + tracks, encoding="utf-8")
        run("check", invalid, fail=True)
    run("render", smart, "-o", OUT / "smart.mp4", "--overwrite")
    subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(OUT / "smart.mp4"),
                    "-f", "null", "-"], check=True)
    report = {
        "default_smart_and_explicit_configuration_order": "passed",
        "early_quiet_tail_intro_and_local_lufs": "passed",
        "retained_material_and_no_three_track_overlap": "passed",
        "actual_pcm_hole_reduction": {"fixed_minimum_half_second_rms_db": old_minimum,
                                      "smart_minimum_half_second_rms_db": minimum},
        "exact_full_pcm_excerpt_and_peak_ceiling": "passed",
        "effects_change_actual_pcm_and_can_be_disabled": "passed",
        "invalid_controls_rejected": "passed",
        "default_smart_h264_aac_mp4_full_decode": "passed",
    }
    (OUT / "verification.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    (ROOT / "docs/research/smart-transition-validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
