# Audorchid 依赖与许可证

更新：2026-10-03。Audorchid **自有代码、文档与原创演示素材使用 Apache-2.0**，根目录已提供 [LICENSE](../LICENSE) 与 [NOTICE](../NOTICE)。依赖、字体和参考来源保留各自协议；项目采用 Apache-2.0 不意味着所有依赖被重新许可。

## 已接入的依赖

| 依赖 | 版本 / 提交 | 实际用途 | 许可 |
| --- | --- | --- | --- |
| GPUI / gpui_wgpu | 27d0daf84e4956f93aac222985e91181ff3d43b5 | HeadlessAppContext、真实文字塑形、Scene、wgpu renderer | Apache-2.0 |
| wgpu | 30.0.1，经 gpui_wgpu 使用 | 离屏纹理、复制、映射读回 | MIT OR Apache-2.0 |
| openh264 / sys2 | 0.9.8 | 源码构建 OpenH264；I420 → H.264 | BSD-2-Clause |
| muxide | 0.2.5 | Annex B + ADTS → MP4 | MIT OR Apache-2.0 |
| oxiaudio / decode / core | 0.2.1 | PCM 类型与音频解码；关闭 facade 的额外编码/DSP功能 | Apache-2.0；内部 Symphonia 保留 MPL-2.0 |
| fdk-aac | 0.8.0，本地小补丁 | 双声道 AAC-LC 编码与 EOF drain | Rust wrapper 为 MIT |
| fdk-aac-sys | 0.5.0 | 原生 FDK AAC 编码器 | sys wrapper 元数据为 MIT；C 库另有专用许可 |
| rubato | 5.0.1 | 按需 FFT 重采样与延迟补偿 | MIT OR Apache-2.0 |
| rustfft | Cargo.lock 固定 | 4096 点因果 Hann 频谱、双声道分析 | MIT OR Apache-2.0 |
| ebur128 | 0.1.10 | 门限综合 LUFS、过采样真峰值 | MIT |

完整解析版本见 Cargo.lock。[依赖清单](research/dependency-licenses.json)记录当前 Windows 构建/测试依赖图与收集到的许可文本，包含构建和测试依赖，不是只列最终链接项。收集脚本为 `scripts/collect_licenses.py`。部分 crate 发布包缺少许可全文，索引中 notices 为空的条目仍需在正式发行时核查；不能仅凭 SPDX 元数据宣称发行审计完成。

当前已在 Rust 1.96.1、Windows x86_64 MSVC 工具链完成构建。GPUI 使用其固定 async-task fork。OpenH264 与 FDK 从源码编译；程序运行时没有调用 FFmpeg，也没有引用 BASS。

## Apache-2.0 的适用边界

保留 MIT/BSD/Apache/OFL 等依赖的原许可，同时将音兰自有代码置于 Apache-2.0。参考项目的公开源码不自动授予任意复制权；LiveSort 没有找到许可证，因此没有把其代码纳入项目。交叉淡化按通用数学公式独立实现。

VLC 的 effects.c 为 LGPL-2.1-or-later；本轮阅读其频谱/示波器原理后独立实现，没有复制 VLC 代码或数据表，也没有把 VLC 加入运行依赖。新增 ebur128 采用 MIT，其声明已收集到 licenses/third-party/ebur128-0.1.10/LICENSE。更新后的 Windows 构建/测试图记录 485 个第三方包、817 份许可文本；dasp_frame/sample 发布包缺少的 MIT 全文已从各自 .cargo_vcs_info.json 对应提交补齐，来源在同目录 SOURCE.txt。其他缺少全文的条目仍保留在索引中等待正式发行审查。

GPU 离屏读回文件基于 GPUI 源码改写，已在文件头标明固定来源、原版权和修改；上游 Apache 声明已收集。音兰的 NOTICE 描述实际复用与参考关系，不用一份 NOTICE 代替所有第三方许可全文。Apache 第 4 节的分发、修改标记和适用 NOTICE 要求见[官方正文](https://www.apache.org/licenses/LICENSE-2.0)。

FDK AAC 是主要的额外许可条件：其原生 C 库采用[专用许可](../licenses/FDK-AAC-NOTICE.txt)，要求保留声明，并在二进制分发时提供相应源代码；该许可没有授予专利权。因此不能把“Rust wrapper 是 MIT”理解为“底层 AAC codec 是 MIT”，也不能承诺所有商业发行用途均已获得编码专利授权。

项目包含完整未修改的 `fdk-aac-sys 0.5.0` 源码归档（含 bundled C codec）、修改后的 Rust wrapper 及补丁说明，见 [third_party/SOURCES.md](../third_party/SOURCES.md)。当前没有对外发布二进制；正式打包须把适用声明和源码获取材料一同提供。

## 字体与素材

- Roboto Regular 来自用户指定的 roboto-2 固定提交，Apache-2.0。附 `assets/fonts/Roboto-LICENSE.txt`，十个数字的 advance 已核验一致。
- Dream Han Sans CN W10 来自固定发布包，保留未修改 TTF 与 `assets/fonts/DreamHan-OFL.txt`。只用于 CJK 字符/全角标点；本轮已完成真实 GPU 字体渲染。
- 生成的演示 WAV 是本项目合成音，PNG 是本项目程序生成的图案；没有打包他人歌曲、专辑封面或 Apple 图标。
- 字体与 FDK 归档 SHA256 见 [来源记录](../third_party/SOURCES.md)。

## 仅供研究的来源

Apple Music demo、osu/osu-framework（MIT）用于理解组件与视觉表现；本次未复制其 UI 或波形代码。libyuv（BSD-3-Clause + PATENTS）用于研究像素顺序和色彩转换，当前无 libyuv FFI/二进制依赖。LiveSort 仅作行为研究。所有阅读路径、固定提交及结论仍保留在[源码阅读记录](source-review.md)。
