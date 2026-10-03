"""Exercise real CLI previews, temporary paths/scaling, and overwrite guards.

Requires the release binary, demo assets, Pillow, ffprobe and ffmpeg.
FFmpeg is an independent decoder here; Audorchid never invokes it.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/release/audorchid.exe"


def run(*args, cwd=ROOT, error=None):
    result = subprocess.run([str(a) for a in args], cwd=cwd, capture_output=True)
    stderr = result.stderr.decode("utf-8", errors="replace")
    if error is not None:
        assert result.returncode != 0 and error in stderr, stderr
    elif result.returncode:
        raise RuntimeError(stderr)
    return result.stdout


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    source = (ROOT / "examples/audorchid.en.toml").read_text(encoding="utf-8")
    source = source.replace("../assets/", (ROOT / "assets").as_posix() + "/")
    source = source.replace('"media/', '"' + (ROOT / "examples/media").as_posix() + "/")
    source = source.replace("crossfade_seconds = 2.0", "crossfade_seconds = 0.5")
    source = source.replace("# trim_end_seconds = 8.0", "trim_end_seconds = 2.0")
    # Both tracks must be short, including the second one without a trim example.
    source = source.replace('title = "月下 / Moonrise"', 'trim_end_seconds = 2.0\ntitle = "月下 / Moonrise"')
    report = {"checks": []}
    with tempfile.TemporaryDirectory(prefix="audorchid-cli-") as folder:
        directory = Path(folder)
        configured_video = directory / "configured.mp4"
        source = source.replace("../output/audorchid.mp4", configured_video.as_posix())
        config = directory / "project.toml"
        config.write_text(source, encoding="utf-8")
        config_hash = digest(config)

        # No --seconds or --output: first frame and derived path really work.
        run(BINARY, "preview", config)
        first = directory / "configured.preview.png"
        assert Image.open(first).size == (1920, 1080)
        run(BINARY, "frame", config, "-o", "alias.png", cwd=directory)
        assert first.read_bytes() == (directory / "alias.png").read_bytes()
        report["checks"].append("first frame, derived filename, frame alias, terminal-relative output")

        run(BINARY, "preview", config, error="output exists")
        run(BINARY, "preview", config, "--overwrite")
        assert first.read_bytes() == (directory / "alias.png").read_bytes()
        for flag, expected in [("0", "--scale"), ("1.5", "--scale"), ("nan", "--scale")]:
            run(BINARY, "preview", config, "--scale", flag, error=expected)
        run(BINARY, "preview", config, "--seconds", "3.5", "-o", directory / "late.png",
            error="--seconds must be within")
        run(BINARY, "preview", config, "-o", directory / "wrong.mp4", error="must end in .png")
        cover = ROOT / "examples/media/first.png"
        cover_hash = digest(cover)
        run(BINARY, "preview", config, "-o", cover, "--overwrite", error="overwrite an input")
        assert digest(cover) == cover_hash
        report["checks"].append("explicit overwrite, invalid parameters, input protection")

        run(BINARY, "preview", config, "--seconds", "0.75", "--scale", "0.5", "-o", directory / "half.png")
        assert Image.open(directory / "half.png").size == (960, 540)
        plan = json.loads(run(BINARY, "inspect", config))
        assert plan["audio_settings"]["crossfade_curve"] == "equal_power"
        geometry = plan["layout"]
        custom = source.replace('preset = "16:9"', 'preset = "custom"')
        custom = custom.replace('progress_position = "under_artist_lower"', 'progress_position = "auto"')
        custom_fields = "\n[layout.custom]\n" + "\n".join(
            f"{key} = {json.dumps(value)}" for key, value in geometry.items()) + "\n"
        custom = custom.replace("\n[video]", custom_fields + "\n[video]")
        custom_config = directory / "custom.toml"
        custom_config.write_text(custom, encoding="utf-8")
        run(BINARY, "preview", custom_config, "--seconds", "0.75", "--scale", "0.5",
            "-o", directory / "custom-half.png")
        assert (directory / "half.png").read_bytes() == (directory / "custom-half.png").read_bytes()
        portrait = source.replace('preset = "16:9"', 'preset = "3:4"')
        portrait = portrait.replace("width = 1920", "width = 1080").replace("height = 1080", "height = 1440")
        portrait_config = directory / "portrait.toml"
        portrait_config.write_text(portrait, encoding="utf-8")
        run(BINARY, "preview", portrait_config, "--scale", "0.5", "-o", directory / "portrait.png")
        assert Image.open(directory / "portrait.png").size == (540, 720)
        report["checks"].append("half-size landscape, matching custom layout, 3:4 portrait")

        video = directory / "temporary.mp4"
        run(BINARY, "render", config, "--scale", "0.1666666667", "-o", "temporary.mp4", cwd=directory)
        assert not configured_video.exists()
        probe = json.loads(run("ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", video))
        v = next(s for s in probe["streams"] if s["codec_type"] == "video")
        a = next(s for s in probe["streams"] if s["codec_type"] == "audio")
        assert (v["width"], v["height"], v["codec_name"], a["codec_name"]) == (320, 180, "h264", "aac")
        assert v["nb_frames"] == "105" and abs(float(a["duration"]) - 3.5) < 1e-6
        run("ffmpeg", "-v", "error", "-i", video, "-f", "null", "-")
        before = digest(video)
        run(BINARY, "render", config, "-o", video, error="output exists")
        assert digest(video) == before
        run(BINARY, "render", config, "--scale", "0.1666666667", "-o", video, "--overwrite")
        run("ffmpeg", "-v", "error", "-i", video, "-f", "null", "-")
        assert digest(config) == config_hash and not configured_video.exists()
        assert not list(directory.glob(".audorchid-*"))
        report["checks"].append("MP4 override, codecs/duration, decode, overwrite, unchanged TOML, temp cleanup")
        report["video"] = {"width": v["width"], "height": v["height"], "frames": int(v["nb_frames"]),
                           "audio_seconds": float(a["duration"])}
    (ROOT / "docs/research/cli-validation.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
