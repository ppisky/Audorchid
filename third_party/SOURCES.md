# Third-party source records

Audorchid's original code is Apache-2.0. These components keep their own licenses.

| Component | Exact source | Local record |
| --- | --- | --- |
| GPUI / gpui_wgpu | [gpui-fusion 27d0daf8](https://github.com/cradiy/gpui-fusion/tree/27d0daf84e4956f93aac222985e91181ff3d43b5) | Cargo.lock; renderer adapter carries origin and modification notices |
| fdk-aac wrapper | [0.8.0 source](https://crates.io/api/v1/crates/fdk-aac/0.8.0/download) | vendor/fdk-aac; MIT; see AUDORCHID-PATCHES.md |
| fdk-aac-sys + native codec | [0.5.0 complete crate source](https://crates.io/api/v1/crates/fdk-aac-sys/0.5.0/download) | sources/fdk-aac-sys-0.5.0.crate; includes the complete unmodified bundled C source and its NOTICE |
| Roboto Regular | [roboto-2 38062f4b](https://github.com/googlefonts/roboto-2/tree/38062f4b4a0be4346d07a928408da21602545e9e) | assets/fonts/Roboto-Regular.ttf; Apache-2.0 |
| Dream Han Sans CN W10 | [dream-3.03-sans-2.005-serif-2.003 release](https://github.com/Pal3love/dream-han-cjk/releases/tag/dream-3.03-sans-2.005-serif-2.003) | One unmodified TTF extracted from DreamHanSansCN.zip; SIL OFL 1.1 |

SHA256:

```text
fdk-aac-sys-0.5.0.crate 24516d2611506d5cb1833555adc75f6baf9fe2706b9c13e6fc33a6b22c51ca83
Roboto-Regular.ttf 56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d
DreamHanSansCN-W10.ttf 20b886d29306e802d3726bd97fcdf17676997c9c91ae954bf1339a13cca5f561
```

The FDK source archive is a gzip-compressed tar archive (`.crate`). Keep it, the
wrapper modifications, and the native NOTICE available with binary distributions.
Audorchid has not modified the native C library. The FDK license does not grant
patent rights; the project's license does not override those terms.

Other Cargo package versions and checksums are fixed by Cargo.lock. The current
Windows build/test graph and collected license texts are indexed in
docs/research/dependency-licenses.json. Metadata labels alone are not a complete
binary distribution license audit; some upstream packages omit license text
from their crate archives. This repository has not published a binary release.
