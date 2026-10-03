# Audorchid 音兰

TOML 配置驱动的 Rust 离线音乐可视化渲染器。通过 GPUI + wgpu 绘制封面、歌曲信息、波形和三角形动画，使用 OpenH264、FDK AAC 和 muxide 导出 H.264 + AAC-LC 的 MP4。

内置 **16:9、9:16、1:1、3:4** 四种固定版式，也可明确填写自定义坐标。曲目按时间线自动衔接；成片中没有播放、暂停或切歌按钮。

预设使用紧凑标题/作者、白色小进度条、稍圆的封面与底部白色宽柱频谱，默认不画三角形。频谱按区域宽度自适应：16:9 为 80 根、9:16 为 40 根、1:1 与 3:4 为 43 根。浅黄色标出实际交叠区间；颜色、圆角、位置、尺寸和疏密都能手调。

日常可以只写音乐路径，字体、输出、标题、音频处理和版式都有默认值：

```toml
[audio.arrangement]
enabled = true # false：按 TOML 顺序播放

[audio.transitions]
enabled = true # false：关闭融合，没有黄色区

[[tracks]]
audio = "music/first.flac"

[[tracks]]
audio = "music/second.flac"
```

标题省略时用文件名，封面省略时生成占位图。默认自动编排、逐曲 −16 LUFS、smart 自动衔接，输出 1920×1080/30 fps；字体直接内置。用 CLI `-o` 选择成片路径，省略时在 TOML 旁生成 `audorchid.mp4`。用 `[audio.arrangement] enabled = false` 关闭自动编排；用 `[audio.transitions] enabled = false` 关闭融合和黄色区。旧的 configuration 模式和 crossfade_seconds=0 继续支持。默认值的研究依据与覆盖规则见[默认值设计](docs/defaults.md)。

## 快速运行

当前验证环境为 Windows x86_64、Rust 1.96.1、MSVC Build Tools、可运行 wgpu 的 GPU/驱动。首次构建会下载固定版本的依赖并编译原生编码器。导出过程不调用 FFmpeg，不需要音频设备；FFmpeg 仅用于开发验收。

在项目根目录运行：

```powershell
cargo build --release --locked
./target/release/audorchid demo
./target/release/audorchid check examples/audorchid.simple.zh-CN.toml
./target/release/audorchid preview examples/audorchid.simple.zh-CN.toml --scale 0.5 -o output/preview.png
./target/release/audorchid render examples/audorchid.simple.zh-CN.toml -o output/audorchid.mp4
```

`demo` 创建两首原创 16 秒短曲、MIDI 乐谱与封面：电钢琴和声、拨弦旋律、贝斯、鼓与立体声延迟，采用程序自制采样库，无外部歌曲或采样依赖。完整版特意固定交叠 2 秒（一小节），成片 30 秒；简洁版使用 smart 自动规划，时长取决于实际交叠。详见[演示音乐](docs/demo-music.md)。

`demo` 拒绝覆盖已有素材；已有旧音频时可先备份再重新生成，或用 `demo --dir output/new-demo` 生成到新目录并修改 TOML 路径。已有输出默认不会覆盖；可临时加 `--overwrite`，或给视频配置 `overwrite = true`。预览始终需要 CLI 的 `--overwrite` 才会替换已有 PNG。

- 日常用[简洁中文 TOML](examples/audorchid.simple.zh-CN.toml) · [Simple English TOML](examples/audorchid.simple.en.toml)。
- 完整[中文 TOML](examples/audorchid.zh-CN.toml) · [English TOML](examples/audorchid.en.toml)：相同字段和值，逐项解释填写方法与单位。
- [配置与固定坐标](docs/configuration.md)：换比例、自定义布局、裁剪和衔接规则。
- [实现计划与范围](docs/plan.md) · [源码阅读记录](docs/source-review.md)。
- [依赖与许可](docs/dependencies-and-licensing.md) · [验收记录](docs/validation.md)。

## 命令

```powershell
# 完整解码并检查时间线；不启动 GPU
./target/release/audorchid check examples/audorchid.zh-CN.toml
# JSON 输出：实际样本数、曲目起点、帧数、固定布局
./target/release/audorchid inspect examples/audorchid.zh-CN.toml
# 只生成首帧 PNG：默认 output/audorchid.preview.png，不编码视频
./target/release/audorchid preview examples/audorchid.zh-CN.toml
# 临时半尺寸预览指定时刻；保留原比例、同步缩放布局与字体
./target/release/audorchid preview examples/audorchid.zh-CN.toml --seconds 4.2 --scale 0.5 -o output/preview.png
# 临时指定成片文件名，不改 TOML；也可加 --scale 0.5 导出快速草稿
./target/release/audorchid render examples/audorchid.zh-CN.toml -o output/my-video.mp4
# 只听准备好的混音片段，不渲染视频
./target/release/audorchid audio-preview examples/audorchid.zh-CN.toml --seconds 12 --duration 12 -o output/listen.wav
# 直接试听第 1 处衔接，默认前后各 6 秒；可选 --transition 2 --before 3 --after 8
./target/release/audorchid transition-preview examples/audorchid.simple.zh-CN.toml -o output/handoff.wav
# 独立生成类似默认占位图的封面；同一 seed 的结果固定
./target/release/audorchid cover -o output/generated-cover.png --size 1600 --seed 42
# 运行逻辑、音频及封装测试
cargo test --workspace --locked
```

TOML 内路径相对于配置文件；CLI `-o/--output` 相对于终端当前目录。`--scale` 范围为大于 0 且不超过 1，会取保持准确比例的偶数尺寸；预览/导出会报告实际分辨率，TOML 不变。旧 `frame` 命令作为 `preview` 的别名保留。封面支持 PNG/JPEG；Roboto Regular 和 Dream Han Sans CN W10 内置于可执行文件，也可用 fonts.latin/cjk 路径覆盖，无需安装到系统。无损 WAV/FLAC 是首选输入；mono 自动复制为 stereo，其他声道数明确拒绝。

## 当前版本

首版已实现配置校验、首帧/指定时刻 PNG 预览、临时输出与分辨率参数、四种预设与自定义坐标、PCM 裁剪/重采样/混音、可调曲线交叉淡化、逐曲/整段 LUFS 与真峰值保护、示波线/包络波形、带峰值帽的频谱、可调实心三角形、逐曲进度与黄色交叠提示、可选自动歌单编排、真实字体排版、画质优先编码、GPU 离屏读回、BT.709 I420、H.264/AAC 编码与 MP4 延迟编辑。

`audio.transitions.enabled = false` 时直接拼接，所有交叠长度为零，不添加动态效果，也不画黄色区域。默认开启时，`smart` 扫描安静尾巴/前奏、逐次选择提前混入的时间，并进行有界局部响度补偿、频段交接和温和立体声扩散；效果平滑回到原声。`adaptive` 保留上一版仅搜索长度的方案，`fixed` 才按 `crossfade_seconds` 指定秒数，0 在所有模式下关闭交叠。所有实际长度、章节和黄色区间共用时间线。详见[衔接与快速试听](docs/transitions.md)。自动编排可独立改为按 TOML 顺序；原素材不裁剪/变速，尚未实现乐句、人声切点或节拍同步。

封面省略时使用原创程序生成的圆环/渐变图，可选 `tracks.cover_seed` 固定变化。`cover` 命令可独立导出 64～4096 px 的正方形 PNG，默认 1600 px；不需要项目、音乐或 GPU。提供封面时按文件内容识别 PNG/JPEG，也支持实际是 PNG 但扩展名为 .jpg 的下载文件。

目前使用同步 GPU 读回和内存 PCM/压缩包，适合先验证短片。muxide 编码视频缓存达到 1.5 GB 时会停止；长歌单的流式缓存与增量封装、歌词、硬件编码和桌面预览仍在后续计划中。跨平台构建尚未验证。

## 开发与 CI

见 [贡献指南](CONTRIBUTING.md)。GitHub Actions 在每次推送/PR 检查格式、发布内容与私有素材排除规则，并在 Windows 构建、运行 Clippy/测试和真实音频衔接试听回归。GPU 渲染用例需在有可用显卡/驱动的本地环境显式运行，CI 不把跳过的 GPU 用例计作通过。

## 开源协议

Audorchid 自有代码采用 [Apache-2.0](LICENSE)，提供 [NOTICE](NOTICE)。第三方代码和字体保留原许可；FDK AAC 原生库采用专用许可，其完整声明与源码获取方式见 [许可记录](docs/dependencies-and-licensing.md)。项目的 Apache-2.0 不能替代第三方编码专利授权。
