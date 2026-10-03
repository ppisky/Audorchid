"""Check tracked publication contents; no network or third-party Python packages."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = (".research/", "target/", "output/", "cache/", "examples/media/", ".env")
SOURCES = {
    "assets/fonts/Roboto-Regular.ttf": "56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d",
    "assets/fonts/DreamHanSansCN-W10.ttf": "20b886d29306e802d3726bd97fcdf17676997c9c91ae954bf1339a13cca5f561",
    "third_party/sources/fdk-aac-sys-0.5.0.crate": "24516d2611506d5cb1833555adc75f6baf9fe2706b9c13e6fc33a6b22c51ca83",
}


def main():
    result = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    paths = [p.decode("utf-8") for p in result.split(b"\0") if p]
    assert paths, "no tracked files to inspect"
    required = ["LICENSE", "NOTICE", "Cargo.lock", "assets/fonts/Roboto-LICENSE.txt",
                "assets/fonts/DreamHan-OFL.txt", "licenses/FDK-AAC-NOTICE.txt", *SOURCES]
    assert all(p in paths for p in required), "required source/license record is not tracked"
    for name in paths:
        assert not name.startswith(FORBIDDEN), f"generated/private file tracked: {name}"
        assert not (name.startswith("assets/") and not name.startswith("assets/fonts/")), f"private asset tracked: {name}"
        assert (ROOT / name).stat().st_size < 50_000_000, f"oversized file: {name}"
        if name.startswith(("crates/", "docs/", "examples/")) and name.endswith((".rs", ".md", ".toml", ".json")):
            content = (ROOT / name).read_text(encoding="utf-8-sig")
            normalized = content.replace("\\\\", "\\").replace("/", "\\")
            assert "C:\\Users\\Administrator" not in normalized and "D:\\AI\\Audorchid" not in normalized, f"local machine path: {name}"
    for name, digest in SOURCES.items():
        assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, f"source hash changed: {name}"
    records = json.loads((ROOT / "docs/research/dependency-licenses.json").read_text(encoding="utf-8-sig"))
    count = 0
    for package in records["packages"]:
        for notice in package["notices"]:
            path = notice["path"]
            assert path in paths, f"notice not tracked: {path}"
            assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == notice["sha256"], f"notice hash changed: {path}"
            count += 1
    print(f"Publication contents checked: {len(paths)} tracked files; private assets excluded; {count} notices and licensed source hashes intact")


if __name__ == "__main__":
    main()
