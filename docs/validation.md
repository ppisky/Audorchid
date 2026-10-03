# 验收记录

## 2026-10-03：衔接预览、明确开关与源码发布准备

- 新增 `transition-preview`，按实际编排后的 1 起始编号选择衔接；默认包含整个交叠及前后各六秒，支持 CLI 上下文与输出覆盖，省略输出时自动派生 WAV 名。关闭融合仍可预览直接拼接，越界、无效时间、空范围与覆盖源文件均拒绝。
- 新增 arrangement.enabled 与 transitions.enabled，默认均 true。前者 false 强制 TOML 顺序；后者 false 优先关闭所有交叠与效果。旧的 configuration 模式和 crossfade_seconds=0 继续支持。中英文简洁与完整两对示例均明确展示开关，解析值一致。
- 动态混音跳过会被覆盖的基础交叠，保证每处只处理一次；关闭交叠时跳过端点规划与局部响度工作；WAV 使用有界缓冲写入。私有音乐第一处新 CLI 试听与上一轮对应最终 PCM 片段 SHA-256 完全相同，整理未改变声音。
- workspace 31 项测试通过，格式与 Clippy 全 workspace/all-targets `-D warnings` 通过。Release 和开发构建通过。CPU CLI 回归验证编号、上下文、派生输出、逐样本相等、错误参数及两个开关；实际 GPU 图像检查关闭衔接时 show_overlap=true/false 的 RGB 完全相同，开启时有标记差异。四比例默认布局/柱数检查继续通过。
- 源码发布检查使用真实 Git 索引，排除私有 assets（仅 fonts 例外）、研究缓存、生成演示与输出；817 份许可声明和字体/FDK 源码 SHA-256 与来源记录一致。许可证文件保持原始字节，代码/文档统一 LF。Git 作者与提交者均采用 ppisky 的 GitHub noreply 身份，未使用本机全局邮箱。
- GitHub Actions 分别执行格式/发布内容检查，以及 Windows 锁定工具链构建、Clippy、测试和真实 CPU 音频 CLI 回归。GPU 用例仍需本地显式运行；不将 ignored 计为通过。见 [工作流](../.github/workflows/ci.yml) 与 [贡献指南](../CONTRIBUTING.md)。

## 2026-10-03：提前安静检测与默认 smart 动态衔接

- 重新逐段核读 LiveSort 尾部扫描、无效尾部排除、局部动态、入场时机、包络、滤波/混响与释放。此前长度搜索不等同于完整适配；默认现改为独立 Rust smart 实现，支持尾部/前奏检测、局部 BS.1770 LUFS、有界补偿、连续频段/空间尾音与原声接回。
- workspace 30 项测试通过，2 项既有 GPU 用例仍为显式 ignored；fmt 与全 workspace/all-targets Clippy `-D warnings` 通过，Release 构建通过。新增用例检查早安静尾部、前奏、短曲容量限制、数字静音、增益上限及处理起止样本连续性。
- `verify_smart_transitions.py` 使用原创合成测试素材执行实际 CLI。默认 smart 与配置顺序、局部 LUFS、素材时长保留、无三曲重叠、完整 PCM/截取逐样本相等、效果关闭、错误参数拒绝及实际 H.264/AAC/MP4 完整独立解码通过。该测试中的固定两秒静音低谷已消除；这仅验证算法行为，不能作为主观听感分数。[机器记录](research/smart-transition-validation.json)
- `verify_transitions_and_covers.py` 原有长度搜索/变量时间线、试听、封面与导出检查继续通过。`verify_defaults.py` 实际验证四比例默认布局/柱数、简洁配置的 smart 默认和明确 fixed 的完整演示；相同 fixed 模式的简洁/完整配置 RGB 和时间线/响度数据一致。[默认值记录](research/defaults-validation.json)
- 私有四首歌曲仍按实际分析顺序 `[3,0,1,2]` 排列，新交叠为 16.25/12.10/12.20 秒。三段试听逐字节等于整段最终 PCM 中的相同样本；独立 FFmpeg 对整段测得约 −15.9 LUFS、−1.1 dBTP。所有八个私有源文件 SHA-256 与先前基线一致、仍被忽略；测量与试听只存 ignored output，不将私有素材放进公共测试记录。
- 实际查看 1080p 交叠 PNG：字体、封面渐变、计数、白色进度、黄色实际交叠与底部频谱正常。字幕/章节仍采用交叠中点策略；不是动态能量相等时刻。原素材不自动裁剪/变速/对拍，尚未实现调性、乐句或人声切点，未宣称听感优于参考。

以下为先前阶段的历史验收；其中“默认 fixed/2 秒”现已由 smart 默认取代。

## 2026-10-03：白色进度、封面圆角、自适应频谱与简洁默认值

- 已播放进度默认改白色，保留浅黄色真实交叠区；外观均可覆盖。封面圆角由 18 加到 24 参考像素，实际查看四比例 PNG 后保留；进度仍保持原位置与间距。
- 自适应目标柱宽 14.5、gap=0.3：16:9/9:16/1:1/3:4 实际柱数为 80/40/43/43，完整频率范围重新分箱；等比缩小预览不改变柱数，显式 bars 固定数量。核心测试还覆盖半宽区域、目标宽与手动覆盖。
- `cargo test --workspace --locked`：26 项通过；Release、fmt、Clippy 全 workspace/all-targets `-D warnings` 通过。
- `scripts/verify_defaults.py` 实际 GPU 验证：简洁版使用内置字体，与显式完整版在相同帧 RGB 逐像素相同，时间线/音频/布局/响度/编排数据相同。只写音频路径也成功预览，标题取文件名。自动模式确实按 `[1,0]` 播放；configuration 为 `[0,1]`。[机器记录](research/defaults-validation.json)
- 四比例宽带噪声实际绘图逐列检测，白色分隔条数量为 80/40/43/43，验证 CPU 频段与 GPU 绘制一致，而非只检查 inspect 打印值；实际查看四种音乐 PNG。预览保存在 output/defaults-checks。
- 首帧/时刻、临时输出/缩小、输入保护、显式覆盖、TOML 不变与临时文件清理的 `verify_cli.py` 再次通过。`verify_gapless.py` 明确 configuration/off，96,017 样本在 48,013 处分段后的解码 AAC 与连续源逐字节相同。
- 新默认视频 `output/audorchid-white-adaptive.mp4`：1920×1080、30 fps、900 帧、30 秒、H.264 + 48 kHz 双声道 AAC，3,027,927 字节、101.80 秒导出；完整独立解码通过。实际查看第 450 帧，从 Moonrise 切到 Morning light，02/02 与当前歌曲计时同步，白色/黄色区正确。[ffprobe](research/white-adaptive-probe.json) · [实际计划](research/white-adaptive-plan.json)
- 中英文简洁与完整两对 TOML 的解析数据分别一致；默认 automatic/track，旧显式 configuration/off/bars/颜色/圆角仍按用户值执行。字体嵌入二进制，默认使用与仓库相同的字节，并保留原字体许可。研究依据及默认行为迁移见[默认值文档](defaults.md)。

## 2026-10-03：CLI 首帧预览、临时输出与可调衔接曲线

- 保留用户认可的视频布局、间距与浅黄色交叠区。新 Release 在 4.2 秒生成的 1920×1080 PNG，与上一轮 `revised-preview-final.png` 的 RGB 像素完全相同；默认声音仍为 2 秒 equal_power / shape=1，新旧默认增益逐项浮点精确相等。
- `cargo test --workspace --locked`：24 项通过；Release 构建、fmt、Clippy 全 workspace/all-targets `-D warnings` 通过。新增曲线测试检查端点、单调性、等功率/线性增益约束、形状极值及默认兼容性；CLI 检查首帧/别名、四比例缩小、自定义几何与素材覆盖保护。
- `scripts/verify_cli.py` 实际执行 Release：默认首帧派生路径与 frame 别名结果逐字节一致；命令行相对输出使用终端目录；960×540 内置与相同 custom 的 PNG 逐字节一致，3:4 半尺寸为 540×720；无效 scale/时刻/后缀被拒绝。PNG 覆盖需要显式参数，覆盖源封面被拒绝且原文件不变。[CLI 记录](research/cli-validation.json)
- 临时 `render -o ... --scale ...` 导出 320×180、105 帧、3.5 秒 H.264 + AAC，完整独立解码通过；原 TOML 指定的视频未被创建，TOML SHA-256 不变。已有文件拒绝覆盖、显式覆盖成功，临时文件清理通过。
- 已实际生成 `output/audorchid.preview.png`（1920×1080 的第 0 帧），以及原尺寸/半尺寸的 4.2 秒预览。首帧的瞬时音频为零，可视化隐藏；检查频谱时使用 --seconds。
- 重新阅读 LiveSort 的尾部 RMS、交叠推导、动态 gain/duck/入场延迟、滤波与混响流程，记录在[源码阅读](source-review.md#livesort-的分析与衔接)。本轮提供独立实现的 equal_power/smooth/linear 与 shape 控制，没有宣称实现其动态效果或自动切点。
- 单独对照 `output/transition-smooth.mp4`：4 秒 smooth，交叠全局 12～16 秒、第 14 秒切换标题，28 秒、960×540、840 帧、H.264 + 48 kHz 双声道 AAC，1,752,341 字节，32.05 秒导出。完整独立解码通过；解码 PCM 的前 11.5 秒与原 2 秒版逐字节相同，12.5～13.5 秒已经不同，验证较早混入。实际查看第 420 帧，标题、02/02、00:02/00:16 及黄色区间一致。原默认视频保留。[对照验证](research/transition-comparison-validation.json) · [ffprobe](research/smooth-transition-probe.json) · [原尺寸计划](research/smooth-transition-plan.json)
- 对照配置 `output/transition-smooth.toml` 写明参数与 CLI 缩小方式；两段 WAV 试听均从对应成片的全局 10～18 秒解码，便于直接比较。中英文主示例新增曲线/形状和 CLI 注释，解析字段/值保持一致。

## 2026-10-03：字形裁切、逐曲进度、自动编排与导出画质

- 确认之前 PNG 的 `Morning light` 下伸部被固定文字 mask 裁切；此前仅目测布局的验收没有覆盖此问题。本次实际字体回归先验证旧行框会越界，再检查 Roboto/Dream Han 混排、g/y/p/q/j、四种标题字号、1×/2× DPI 与小尺寸文字。按真实栅格 bounds 修正基线；小尺寸使用可容纳的安全边距，消除浮点相等边界造成的误报。
- `cargo test --workspace --locked`：20 项通过；Release、fmt、Clippy/all-targets `-D warnings` 通过。
- `scripts/verify_revision.py`：四种原尺寸 GPU PNG、曲尾与曲首黄色区、整段模式、零交叠无黄色、缩小进度后的计数右对齐通过。实际右边缘误差 0.24 像素。默认横屏 `[870,588,772.8,4]` 保留作者左对齐；移除黑色章节刻度。[量化记录](research/visual-revision-validation.json)
- 两项显式 GPU 测试通过（63.51 秒）：自动顺序确实从 `[0,1]` 变为 `[1,0]`，其 PCM 与手工重排逐样本相同，起点、449/450/451/600 帧画面逐像素相同；连续帧与冷启动切歌帧相同。之后的小字号边距修复已对比默认 1080p 原始帧，像素保持一致。
- 15 秒属于首尾分析窗口；默认交叠仍为 2 秒。16+16 秒的演示在全局第 15 秒切换到第二首，此时歌曲计时为 `00:01 / 00:16`。没有把两首歌合成默认进度条；program 模式显式可选。
- 默认画质改为 OpenH264 Quality、QP 18、High complexity；实际 QP 18/38 编码结果不同，避免 RC_OFF 忽略范围的问题。去掉导致中间调变暗的额外亮度曲线变换；256 级灰阶 limited-range 往返误差不超过 1。
- 独立 FFmpeg 解码对照同一 1080p 场景：QP 18 全画面 RGB PSNR 43.226 dB、标题亮度 PSNR 43.782 dB；码率模式标题亮度 39.491 dB；QP 38 为 32.262 dB。RGB 标题区受 4:2:0 色度平均影响，单独报告亮度细节，未将有损编码描述为无损。
- `output/audorchid-revised.mp4`：1920×1080、30 fps、900 帧、30 秒、H.264 + 48 kHz 双声道 AAC，2,993,468 字节，111.12 秒导出（并行验收时测得）。完整独立解码通过；实际查看了 126 帧和 449/450 帧，确认清晰度、计数及黄色区域切换。[ffprobe](research/revised-video-probe.json)
- 最终 Release 复测 `verify_media.py` 通过：四种缩小比例、custom、重复取帧、AAC 首尾脉冲均通过，脉冲偏移 `[0,0]`；`verify_gapless.py` 的 96,017 样本在 48,013 处分段后，解码 AAC 与整段源逐字节一致。
- 中英文 TOML 字段/值一致，补充顺序模式、15 秒分析窗口、能量曲线、进度范围/位置/颜色、计数内缩、三类圆角、背景虚化、画质与码率注释。


## 2026-10-03：默认无三角形、样式预设与音乐演示

- Release 构建、15 项 workspace 测试、fmt、Clippy/all-targets `-D warnings` 通过。版式测试扩展到四种比例 × 两种可视化位置 × 四种进度位置，包含封面右侧位置及偏移后的边界检查。中英文配置数据相同。
- `python scripts/verify_demo.py`：两首 16 秒 stereo WAV，44.1/48 kHz，样本峰值均约 0.800，无截幅；配套 MIDI 格式 1、5 轨、480 PPQ、120 BPM，每首 233 个音符事件、26 种音高，所有 note-on/off 成对，五轨都在 32 拍结束；重复生成 WAV/MIDI 逐字节相同。[机器记录](research/demo-music-validation.json)
- `python scripts/verify_triangle_styles.py`：实际 GPU 检查省略 triangles 与显式 0 一致，单填样式不会开启；opacity=0 和 border thickness=0 逐像素等于背景。layered/border 均可见，thickness=1 填满的面积明显大于细边框，重复取帧一致。[机器记录](research/triangle-styles-validation.json)
- 已实际检查默认横屏、封面右侧进度、两种三角形预设，以及 9:16、1:1、3:4 PNG。默认没有三角形，80 根柱形比旧版更宽；3:4 的 cover_side 使用作者下方偏低位置，没有移出画布。[实际布局和响度](research/music-layouts.json)
- 显式 GPU 连续帧测试通过（59.75 秒）：章节中点由真实样本时间线计算，在第 450 帧（15 秒）切换；连续渲染与冷启动取帧一致。另显式启用 layered/border 预设，分别验证跳转前后的确定性。
- 最终 `output/audorchid-music-default.mp4`：1920×1080、30 fps、900 帧、30 秒、H.264 + 48 kHz 双声道 AAC，2,047,624 字节，105.49 秒导出。完整独立解码通过；解码第 450 帧的第二首标题、章节计数与中点进度一致。混音编码前为 -16.35 LUFS / -2.98 dBTP。[ffprobe](research/demo-music-probe.json)
- 原有演示素材已备份到本地 `output/demo-tones-backup`，示例使用 `examples/media` 中的新 WAV/MIDI。此后的默认时长为 30 秒；下面旧记录的 13 秒/390 帧只对应旧测试音。

## 2026-10-03：紧凑排版、单色频谱与直接衔接

- `cargo test --workspace --locked`：15 项通过；四种比例 × 两个可视化位置 × 三个进度位置均在画布内，标题/作者间距与小进度位置受检；移除的 classic 配置明确拒绝。中英文 TOML 数据相同。
- `cargo fmt --all -- --check`、Clippy 全 workspace/all-targets `-D warnings`、Release 构建：通过。
- 实际检查四种比例的默认 PNG，以及作者下方贴合频谱、自选浅蓝色并缩小区域的两个 PNG。默认标题单行，作者紧随其后，小进度条位于作者下面；时间与章节计数各占自己的区域。[六组实际布局](research/compact-layouts.json)
- `python scripts/verify_gapless.py`：48 kHz 立体声 WAV 共 96,017 个样本，在第 48,013 个样本（1.0002708 秒，非 AAC 帧边界）处分成两个文件。0 秒交叠、off 响度模式下，分段输入和完整输入导出后的 AAC 解码 PCM **逐字节相同**，有效音频时长也相同。验证范围为同采样率无损输入；不代表具备静音检测、自动选点或所有有损解码器的延迟正确性。[机器记录](research/gapless-validation.json)
- `python scripts/verify_visual_audio.py`：重新通过 scope、envelope、custom_color、white 四项实际 GPU 检查；静音区域与关闭可视化的背景逐像素相同，有声区域可见，反相立体声没有被抵消，重复取帧一致。LUFS、峰值优先和不可测静音检查通过。[最新机器记录](research/visual-audio-validation.json)
- 显式 GPU 连续帧测试重新通过（17.60 秒）：0～197 帧连续生成后，交叠中点第 195 帧与冷启动直接取帧相同；跳转再返回也相同。
- `output/audorchid-compact-landscape.mp4`：13 秒、1920×1080、30 fps、390 帧，H.264 + 双声道 48 kHz AAC，1,974,335 字节；40.64 秒完成导出，完整独立解码通过。实际查看解码后的第 195 帧，第二首标题、02/02 计数及中点进度一致，文字没有重叠。[ffprobe 记录](research/demo-compact-probe.json)

以下为上一轮样式修订和首版的历史记录；其中 classic 彩色样式已移除，当前支持与默认值以配置文档及上面的最新验收为准。

## 2026-10-03：样式、3:4、章节与 LUFS 修订

- `cargo test --workspace --locked`：14 项通过；新增门限 LUFS、静音/短片段、反相立体声频谱和无前瞻验证；补充 3:4、贴合进度、章节末端和参数边界检查。中英文 TOML 解析结果相同。
- Release 构建、`cargo fmt --all -- --check`、Clippy 全 workspace/all-targets `-D warnings`：通过。
- `python scripts/verify_media.py`：四种比例各导出 105 帧、3.5 秒 AAC，独立解码通过；AAC 首尾脉冲偏移仍为 `[0, 0]`；自定义版式和重复取帧通过。[机器记录](research/media-validation.json)
- `python scripts/verify_visual_audio.py`：当时实际 GPU 对比 scope、envelope、classic、white，在初始静音与尾部静音中，可视化区域逐像素等于关闭可视化的背景；有音频时确实绘制，反相左右声道不抵消，重复取帧一致。最新版脚本已将移除的 classic 项替换为 custom_color。
- LUFS：3 秒 1 kHz 立体声正弦原始测得 -19.9932 LUFS；program 模式目标 -16 后，编码前测得 -16.0000 LUFS，独立 FFmpeg 对 AAC 解码后测得 -16.0 LUFS。
- 峰值优先：瞬态脉冲目标 -5 LUFS、最大增益 12 dB；自动峰值衰减 -11.0617 dB 后，输出真峰值 -1.0000 dBTP，综合响度仍为 -37.6847 LUFS，没有为达标而硬削波。静音读数为 null、自动增益为零。
- 边界：真峰值上限约束编码前 PCM。上述硬起止正弦的编码前真峰值 -16.0063 dBTP，AAC 解码后 FFmpeg 为 -12.1 dBTP，说明编码后瞬态峰值确实可能增加；本版没有承诺 AAC 编码后的绝对峰值上限。
- 已实际检查四种比例 PNG，以及填充三角形、经典绿黄频谱/峰值帽、双声道示波线、进度贴合的 PNG。横屏时间文本曾压到封面下沿，已在最终坐标中修正。
- 显式执行 `cargo test -p audorchid-render --test sequence --locked -- --ignored` 通过：1920×1080 连续渲染 0～197 帧，195 帧（6.5 秒交叠中点）与全新 renderer 直接取帧逐像素一致；跳转后再取 195 帧仍一致。该项需要 GPU 和 demo 素材，因此不加入默认无 GPU 测试执行。
- 放大检查最终 MP4 第 194/195/196/197 帧，标题和章节计数在第 195 帧同步变更，进度跨章节保持连续；原始帧文字完整。编码使用 OpenH264 screen-content 模式，关闭面向相机视频的背景分类。
- 本轮最终演示 `output/audorchid-v2.mp4`：1920×1080、30 fps、390 帧、13 秒，H.264 + 双声道 48 kHz AAC，1,996,658 字节；完整独立解码通过。[最终 ffprobe](research/demo-v2-probe.json)。示例逐曲 LUFS + 第二首手动 -1 dB 后，整段为 -16.50 LUFS。

以下保留首版验收记录；其中的三种版式、11 项测试和旧演示文件数字属于修订之前。

## 首版快照

日期：2026-10-03。环境：Windows x86_64 MSVC，rustc/cargo 1.96.1。以下均为实际运行结果；没有以“代码可编译”替代媒体互操作验证。

## 构建与逻辑

- `cargo build --workspace --locked`：通过。
- `cargo build --release --locked`：通过。
- `cargo test --workspace --locked`：11 项通过，覆盖有理帧时钟、交叠长度/切换点、三种版式与缩放、自定义坐标、未知字段/非法数值、中英文配置一致性、重采样长度与脉冲位置、AAC 不完整尾帧、BT.709 色阶与无效 MP4 box。
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：通过。

## 完整 1080p 演示

命令：`audorchid render examples/audorchid.zh-CN.toml`。素材由 `audorchid demo` 生成：第一首 44.1 kHz、第二首 48 kHz，各 8 秒，交叠 3 秒。应用没有调用 FFmpeg。

| 项目 | 独立 ffprobe 结果 |
| --- | --- |
| 容器 | MP4，4,993,482 字节 |
| 视频 | H.264，1920×1080，yuv420p，30/1 fps，390 帧 |
| 色彩 | BT.709 primaries / transfer / matrix，tv limited range |
| 音频 | AAC，48,000 Hz，双声道，612 个编码帧 |
| 视频起点 / 时长 | 0 / 13.000000 秒 |
| 音频起点 / 时长 | 0 / 13.000000 秒 |
| AAC 延迟 / padding | 2048 / 640 样本，起始 skip_samples 被 ffprobe 识别 |
| 单次实际导出耗时 | 33.51 秒，平均约 11.7 视频帧/秒（含初始化与 AAC 编码） |

`ffmpeg -v error -i output/audorchid.mp4 -f null -` 完整解码通过，无音视频解码错误。ffprobe 原始结果见 [demo-probe.json](research/demo-probe.json)。这是当前机器的单次正确性演示，不是跨设备性能保证；同步读回的首版尚未达到实时 1080p 导出。

## 比例、时间戳与音频边界

`python scripts/verify_media.py --binary target/debug/audorchid.exe` 实际完成：

| 场景 | 输出 | 结果 |
| --- | --- | --- |
| 16:9 | 640×360，30000/1001 fps，44.1 kHz | 105 视频帧，154350 有效音频样本，解码通过 |
| 9:16 | 360×640，同上 | 通过 |
| 1:1 | 360×360，同上 | 通过 |
| 自定义布局 | 所有矩形和字号显式设置 | PNG 渲染通过 |
| 确定性 | 先后渲染 0.25 秒、2.5 秒、再 0.25 秒 | 两次相同时刻的 PNG 字节一致；不同时刻不同 |
| AAC 脉冲 | 96017 样本，靠近起点与终点各一个脉冲 | 独立解码检测偏移均为 0 样本，末尾脉冲保留 |
| 输出保护 | 已有视频时再次 render | 拒绝覆盖，原文件字节不变；check 仍可运行 |

测试脚本会创建独立临时目录，不覆盖用户输入；报告见 [media-validation.json](research/media-validation.json)。普通单元测试不要求 GPU；此媒体脚本需要 GPU、ffprobe 和 ffmpeg。

AAC 以完整压缩帧储存。MP4 edit list 给出有效样本长度，但把整条流导出成裸 PCM 的解码器可能仍返回末帧 padding；本机 13 秒示例裸解码为 624640 样本，有效范围为前 624000 样本。提取 PCM 时应遵循轨道时长裁剪，不能用裸解码缓冲长度反推项目时长。本轮未逐个验证所有播放器的尾部编辑行为。

## 视觉核验

实际查看了 1920×1080、1080×1920、1080×1080 的 PNG。中文/英文标题、Roboto 数字、封面、时间与进度均在预定区域，没有播放控制按钮。预览存于本地 `output/preview-landscape.png`、`output/preview-portrait.png`、`output/preview-square.png`。

在核验中修复了三个接入问题：Dream Han family 名称选择、GPUI headless 的默认 2 倍 DPI、sRGB 目标重复色彩编码。另修复实体句柄退出顺序，避免成功导出后被 GPUI leak detector 中断。

## 尚未覆盖

长歌单和高内存压力、多设备/驱动色彩差异、Linux/macOS、更多有损源编码器的 gapless metadata、所有第三方播放器。流式 PCM、增量 MP4、歌词、硬件编码没有被本轮结果宣称为已完成。正式二进制发布前仍需完成实际发行包的第三方许可核查。
