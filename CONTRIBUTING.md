# Contributing to Audorchid

Audorchid's original code, documentation and generated demo material use Apache-2.0. Preserve upstream notices and licenses when changing vendored code, fonts or dependencies. Please do not add copyrighted music or cover images to a pull request.

The verified build target is Windows x86_64 with MSVC Build Tools and Rust 1.96.1. The toolchain is pinned in rust-toolchain.toml. Video and image preview require a working wgpu device; configuration checks and WAV previews do not. Runtime exports do not invoke FFmpeg.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
python scripts/verify_transition_preview.py --binary target/debug/audorchid.exe
python scripts/check_repository.py
```

CI runs these checks using original synthetic audio. The two GPU sequence tests are explicitly ignored by ordinary cargo test; run them locally after generating demo assets when changing rendering:

```powershell
cargo run --locked -- demo
cargo test -p audorchid-render --test sequence --locked -- --ignored
```

For deeper media checks, build release and use scripts/verify_smart_transitions.py and scripts/verify_transitions_and_covers.py; they additionally need Python numpy/Pillow and FFmpeg as an independent decoder. Keep generated results under ignored output/. Place local music and covers under assets/; only assets/fonts/ is tracked. Never force-add private inputs.

Keep Chinese and English example configuration values equivalent, explain units and defaults in their comments, and document any default-behavior change. When changing audio timing, verify the same sample clock drives PCM, progress highlights and metadata handoffs. A disabled transition must produce no yellow overlap marker.

Configure commits with your GitHub-associated email or GitHub noreply address. This project's initial author uses ppisky's GitHub noreply identity; no local machine email is used.
