# Audorchid 源码阅读记录

## 2026-10-03：端点低谷与可选自动交叠

重新逐段读取 LiveSort 的 audio_analyzer.py 15–149、312–373 行，以及 main.py 835–862、864–998、1082–1271 行，追踪尾部有效区间、局部响度档位、入场延迟、实际双路包络、分块音色/混响和末端释放。确认当前的“排序后统一两秒淡化”没有实现这些过程；首尾 15 秒特征分析也不是对拍或最佳切点选择。

另核读 ISMIR 2017《Automatic Playlist Sequencing and Transitions》§4 的强拍/段落候选切点、变速拍点同步与主观评价。根据实际需求独立实现逐次端点能量搜索：adaptive 模式在有界候选长度上比较预测半秒功率低谷，同时惩罚长交叠；该阶段为仅长度方案；现默认 smart 已加入尾部/前奏检测、局部 LUFS、增益/音色/扩散交接，源音频不裁剪/变速。详细公式、限制和试听命令见[衔接文档](transitions.md)。没有复制 LiveSort 的 Python 动态效果实现，也没有把 RMS 连续视为音乐结构连续。

无封面使用原有原创圆环/渐变程序，现将同一生成器公开为 cover CLI 和 tracks.cover_seed；图片解码改为识别内容签名，以支持扩展名不准确的 PNG/JPEG。私人音乐、封面与诊断结果均留在忽略目录，不进入示例或源码阅读记录。

2026-10-03 默认值更新：重新核读 Apple Music demo 封面容器与 LiveSort 排序，并阅读 ISMIR 2017、DAFx 2016 两篇论文及 AES TD1008.1.21-9 的音乐归一化章节。阅读范围、引用、实现与未实现部分见[默认值依据](defaults.md#阅读依据与选择)，未移植论文或参考仓库代码。

本记录说明参考实现中实际读取的关键代码、它们对音兰的启发及尚未验证的部分。阅读日期为 2026 年 10 月 2 日；使用 GitHub 固定提交的原始文件和 crates.io 发布包，不以仓库首页介绍替代实现核验。

这是针对渲染、时间、音频和封装路径的重点阅读，不是全仓库逐行审计。以下保留 2026-10-02 阅读阶段的结论与当时未验证事项；当天没有运行参考项目或编译完整编码工程。

2026-10-03 实现更新：已构建 GPUI + wgpu 离屏应用，下载并加载 Dream Han W10，完成 H.264/AAC MP4 导出和三个比例的独立解码验证。FDK buffer/EOF、AAC edit list、GPUI DPI 与离屏颜色问题已在音兰适配层处理。最新实测与剩余边界见[验收记录](validation.md)；研究清单中的 `build_verified: false` 是前一阶段快照，不代表本轮实现状态。

## 阅读版本

### 2026-10-03：可选叠层与边框三角形

进一步读取同一 osu! 提交的 `Triangles.cs`、`TrianglesV2.cs`、`TestSceneTrianglesV2Background.cs`、`TestSceneTriangleBorderShader.cs` 和 uniform 数据结构。旧版同时控制 ColourLight/ColourDark、正态分布尺寸、尺寸相关速度；V2 用统一 ScaleAdjust、随机速度、Thickness，测试场景提供 0～1 厚度与横/纵渐变调节。V2 的源码注释也说明 seed 不保证跳转时的画面一致。[TrianglesV2](https://github.com/ppy/osu/blob/c834803ea9988159d67e1dc5d99cfaccada685d9/osu.Game/Graphics/Backgrounds/TrianglesV2.cs)、[可视化测试](https://github.com/ppy/osu/blob/c834803ea9988159d67e1dc5d99cfaccada685d9/osu.Game.Tests/Visual/Background/TestSceneTrianglesV2Background.cs)。

音兰据此增加 layered/border 预设和两个色端：前者尺寸/深浅变化，后者统一尺寸、可调向内边框厚度与竖向渐变。使用自行构造的三段环形几何，未移植 osu! shader；0 为不画边框，1 为填满，数值不宣称与上游 shader 逐像素相等。位置、循环和色彩由绝对成片时间计算，支持离线跳转。默认三角形数量改为 0，预设字段不隐式开启效果。

### 2026-10-03：紧凑信息区与单色频谱

再次实际读取 Apple Music demo 同一固定提交的 `now_playing.rs`：标题 14 px，作者 11.5 px，作者上边距 3 px，二者均单行 truncate；下面的 player_bar 区域上边距为 18 px（约 144～180、238～244 行）。此前为两行标题永久预留高度导致短标题与作者分离。本轮内置版式改为单行标题框，框间留约标题字号 0.2 倍的空隙；进度条缩小并放在作者下方，保留底部/贴合选项。只参考排版比例，不加入源项目的播放控制。[固定源码](https://github.com/cradiy/gpui-apple-music-demo/blob/ae4d5facbb29129c80b65e997440814a7e3b6e46/src/ui/components/now_playing.rs#L144)

根据用户澄清，已删除先前新增的 classic 绿黄红频谱及其示例产物。默认是可直接改色的白色细柱，尺寸由四种预设提供，也可调整区域宽高、柱数和间隔。以下 VLC 段落保留实际阅读历史，不表示当前仍提供该彩色样式。

### 2026-10-03：三角形、VLC 与 LUFS 修订

再次读取 osu 同一固定提交的 `Triangles.cs`（旧实现）及 `TrianglesV2.cs`，发现旧实现的 draw node 明确把 shader `Thickness` 设为 `fill = 1f`，`CreateTriangleShade` 在深浅颜色间取值，等边三角形始终朝上、大小影响移动速度。上一版音兰把这种效果画成随机旋转线框，是实现选择错误。本轮改为独立实现的朝上实心三角形、深色半透明叠加和可配样式；保留 outline/both 供主动选择，不再作为默认。[实际填充入口](https://github.com/ppy/osu/blob/c834803ea9988159d67e1dc5d99cfaccada685d9/osu.Game/Graphics/Backgrounds/Triangles.cs#L248)

实际读取 VLC `modules/visualization/visual/effects.c` 的 spectrum_Run（约 87–379 行）和 scope_Run（约 892–954 行）：前者使用 FFT 频段、柱高回落与独立峰值，按高度上色；后者把 PCM 的带符号左右声道值映射到两条轨道。读取后用 `git ls-remote` 固定到提交 `b5eb753828de2bbec9a80bfee829b5d124e99043` 再下载比对，文件 SHA-256 为 `59f42f96332e8c413dfe18eddfc8ac5b1db7c9fd08ab8695317a5e88d930950d`。该文件许可为 LGPL-2.1-or-later；没有复制其 C 代码或频段表。音兰独立编写对数频段/配色/示波线，并改成 100 Hz 音频时钟以支持确定性离线取帧。[固定源码](https://github.com/videolan/vlc/blob/b5eb753828de2bbec9a80bfee829b5d124e99043/modules/visualization/visual/effects.c)

下载并读取 ebur128 **0.1.10** 发布包的 Cargo.toml.orig、src/ebur128.rs 中的模式、输入、门限综合测量与 true_peak 接口。包 SHA-256 为 `e227cc62d64d6fe01abbef48134b9c1f17d470cef1e7a56337ad05b1f81df7f9`，MIT，运行依赖为 Rust 实现；C 参考实现只属于它的测试依赖，不在本项目运行链路里。选择 Mode::I | TRUE_PEAK；先保存综合响度，再补零排空过采样滤波器测尾部峰值，补零不写入实际 PCM。不是拿 RMS 代替 LUFS。[版本源码](https://docs.rs/crate/ebur128/0.1.10/source/src/ebur128.rs)、[API 与过采样说明](https://docs.rs/ebur128/0.1.10/ebur128/struct.EbuR128.html)

核对 [EBU R128 原文](https://tech.ebu.ch/docs/r/r128.pdf)：广播目标是 -23 LUFS。音兰示例选择 -16 LUFS 是产品默认，不声称它是 EBU 规定；实际检测使用门限综合响度。实现用常量增益及真峰值优先衰减，不加入用户未要求的动态压缩。

以下表格与后续旧章节保留初次研究的版本证据；当前实现行为见[配置文档](configuration.md)。

| 来源 | 固定版本或提交 | 备注 |
| --- | --- | --- |
| cradiy/gpui-apple-music-demo | `ae4d5facbb29129c80b65e997440814a7e3b6e46` | demo 本身的快照 |
| cradiy/gpui | `27d0daf84e4956f93aac222985e91181ff3d43b5` | 链接已重定向到 `cradiy/gpui-fusion` |
| ppy/osu | `c834803ea9988159d67e1dc5d99cfaccada685d9` | 三角形与波形使用端 |
| ppy/osu-framework | `a2bd377a8f9b5242fb7b9f264e575b8df8019b8c` | 补充追踪波形真正的实现；未声称与 osu 的 NuGet 构建版本完全对应 |
| LuN3cy/LiveSort | `787239aa90bc97ca0f5e7c2b5964f054e9e422d0` | 音频分析与导出混音 |
| lemenkov/libyuv | `eee459798486116575ed66c6d6609205d445ffb2` | 镜像中的转换实现与头文件 |
| googlefonts/roboto-2 | `38062f4b4a0be4346d07a928408da21602545e9e` | 已归档，仍按用户指定版本检查 |
| Pal3love/dream-han-cjk | `d17bc7737693db37b7eb333f857f4966a7572e40` | 生成脚本、README 与 OFL 文本 |
| openh264 | `0.9.8` | 发布包源码 |
| wgpu | `30.0.1` | 发布包源码；GPUI workspace 声明 `30.0.0` |
| muxide | `0.2.5` | 发布包源码 |
| oxiaudio | `0.2.1` | 另追踪同版本 core、decode、encode 包 |
| fdk-aac | `0.8.0` | 另读取 `fdk-aac-sys 0.5.0` 中 C 头文件与 NOTICE |
| rubato | `5.0.1` | 发布包源码；与旧 0.x 示例 API 有区别 |

完整获取清单、SHA256 和 crate 压缩包校验值见 [source-manifest.json](research/source-manifest.json)。所列版本是阅读基准，不是已经成功编译的依赖组合。

## Apple Music demo 的状态与视觉分层

重点读取 `src/state/playback.rs` 中 `open_player`（752–825 行），`src/ui/components/now_playing.rs` 开头（1–136 行），以及 `lyrics_panel.rs` 的时钟、滚动与定时文字（29–79、356–413、578–593 行）。

demo 用 `PlaybackController` 和队列管理曲目，订阅 `AudioPlayerEvent` 更新界面。`Ended` 事件调用 `skip_next`，确实具备自动切到下一首的路径，但这不能证明样本级无缝混音。音兰应把下一首何时出现提前写进 `RenderPlan`，使用已准备的音频区间，而不是等运行时播放器结束再启动下一首。[自动切换源码](https://github.com/cradiy/gpui-apple-music-demo/blob/ae4d5facbb29129c80b65e997440814a7e3b6e46/src/state/playback.rs#L752)

`now_playing` 把封面、标题、侧面歌词/队列与 `album_glow(...).time(...)` 组合，适合提取为独立音乐展示场景。它用封面与 track ID 判断背景是否更新，并以 `Instant` 计算背景时间；音兰改成源曲目的成片时间。原有按钮、菜单、鼠标回调和 `player_bar` 不进入视频场景。[Now Playing 源码](https://github.com/cradiy/gpui-apple-music-demo/blob/ae4d5facbb29129c80b65e997440814a7e3b6e46/src/ui/components/now_playing.rs#L30)

`LyricsPresentationClock` 通过系统经过时间与播放器位置纠正歌词时间，滚动动画也记录 `Instant`。这是实时播放的处理，离线快慢不一的帧计算会使它失去确定性。可以借用居中布局、缓动和文字状态结构，但替换时钟。逐字文字已经调用 `TimedText.position(context.position)`，保留这一显式时间入口。[歌词面板源码](https://github.com/cradiy/gpui-apple-music-demo/blob/ae4d5facbb29129c80b65e997440814a7e3b6e46/src/ui/components/lyrics_panel.rs#L29)

demo 的 Cargo.toml 把 GPUI 系列依赖固定在 `5fd13e4b6d984c95b1de2fdd6213a8fde0ab89ea`。该 revision 不同于本次 fork 快照，因此不能原样复制 demo 依赖后假定新离屏 API 可用。[依赖声明](https://github.com/cradiy/gpui-apple-music-demo/blob/ae4d5facbb29129c80b65e997440814a7e3b6e46/Cargo.toml#L15)

## GPUI 已有 2D 离屏实现

完整读取 `crates/gpui_wgpu/src/offscreen_renderer.rs`，并读取其部分 GPU 测试、`HeadlessAppContext`、`PlatformHeadlessRenderer`、测试窗口绘制入口和 `CosmicTextSystem` 的字体入口。

`WgpuOffscreenRenderer::new` 创建无窗口 `WgpuContext`，使用 `Rgba8UnormSrgb`、`RENDER_ATTACHMENT | COPY_SRC` 输出纹理。`render_rgba` 绘制 Scene，复制到 `COPY_DST | MAP_READ` buffer，调用 `map_async`，用 `device.poll(Wait)` 等待，逐行去掉 padding 后返回 RGBA，释放映射并 `unmap`。这正好覆盖所需底层数据流。[完整实现](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui_wgpu/src/offscreen_renderer.rs)

当前实现是单 buffer 同步读回，并不是已经完成多帧流水线。`draw_external` 为实例 buffer 扩容时可能要求重试。音兰首版可复用，再增加输出复用、有限重试和多 slot，不能为了吞吐先跳过正确性。

接口消费 `Scene`，不会自动将任意 GPUI 视图变成场景。`HeadlessAppContext::with_platform` 提供真实文字系统和 renderer factory；`PlatformHeadlessRenderer` 还要求 `render_scene`、`render_scene_to_image` 和共享 atlas。需要把 `WgpuOffscreenRenderer` 接到这些接口上。[应用上下文](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui/src/app/headless_app_context.rs#L65)、[renderer trait](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui/src/platform.rs#L1051)

默认平台工厂在非 macOS 返回 `None`，macOS 使用 Metal。音兰要求纯 wgpu，因而应提供自己的适配器。GPUI 测试窗口的绘制路径会把渲染失败记为日志，音兰还需让失败能够终止作业。[平台工厂](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui_platform/src/gpui_platform.rs#L66)、[测试窗口](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui/src/platform/test/window.rs#L299)

`CosmicTextSystem` 可以建立不扫描系统字体的 database，再添加内存字体，并把 GPUI 字体 features 转给 cosmic-text。`TimedText` 使用外部 position 求值，且检查 UTF-8 字节范围和时间顺序，适合确定性歌词；CJK 字符范围不能用 Rust 字符序号代替 UTF-8 byte range。[字体入口](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui_wgpu/src/cosmic_text_system.rs#L82)、[定时文字](https://github.com/cradiy/gpui-fusion/blob/27d0daf84e4956f93aac222985e91181ff3d43b5/crates/gpui_effects/src/timed_text.rs#L607)

现有离屏测试包含真实 GPU 字形图集、透明度、变换与裁剪检查，但标记为需 GPU 的 ignored 测试。阅读测试不等于本机执行通过，也不能由 `gpui_3d` 的 headless 示例推断完整 2D 视图都已接通。

## osu 的三角形

重点读取 `TrianglesV2.cs` 的初始化、更新、粒子生成和绘制（18–279 行）。它使用等边比例约 0.866、相对坐标、依据 elapsed time 更新位置，并以不同速度系数产生变化。DrawNode 将粒子属性转为批量绘制，边界裁剪与轮廓 uniform 分离。[TrianglesV2](https://github.com/ppy/osu/blob/c834803ea9988159d67e1dc5d99cfaccada685d9/osu.Game/Graphics/Backgrounds/TrianglesV2.cs)

源码明确指出：指定 seed 不保证 seek 后状态一致。音兰借鉴几何和层次，不直接保留这种逐帧累积与重生方式；改成按粒子 ID、出生时间、成片时间求值，或使用固定步进及检查点。这样正常顺序渲染、恢复和单帧预览可以共享状态规则。

轮廓 shader 的具体资源未完成逐行追踪，不把本次阅读说成完整 shader 移植。首版三角形可自行用路径或 WGSL 实现并验证边缘质量。

## osu 的波形真正位于 osu-framework

从 osu 中 `WaveformComparisonDisplay` 和波形测试追踪到 `osu.Framework.Audio.Track.Waveform` 与 `osu.Framework.Graphics.Audio.WaveformGraph`，实际读取后者的采样、重采样、归一化和绘制段落。

`Waveform` 先以约 1 ms 的粒度读取左右声道最大绝对振幅，再通过 FFT1024 填入低、中、高频强度。低频 20–100 Hz、中频 100–2000 Hz、高频 2000–12000 Hz，是该实现的特定选择。它使用 BASS 解码，而且读取路径检查设备存在；音兰的离线 PCM 分析不能照搬此依赖。[采样源码](https://github.com/ppy/osu-framework/blob/a2bd377a8f9b5242fb7b9f264e575b8df8019b8c/osu.Framework/Audio/Track/Waveform.cs#L25)

`GenerateResampledAsync` 对已有点做高斯加权重采样，并通过索引乘法避免浮点累加漂移；这不是音频采样率转换。`WaveformGraph` 根据可见宽度决定点数，取消过时计算，缓存频带最大值，在绘制前裁掉不可见范围，用左右振幅形成上下轮廓。[显示重采样](https://github.com/ppy/osu-framework/blob/a2bd377a8f9b5242fb7b9f264e575b8df8019b8c/osu.Framework/Audio/Track/Waveform.cs#L270)、[绘制源码](https://github.com/ppy/osu-framework/blob/a2bd377a8f9b5242fb7b9f264e575b8df8019b8c/osu.Framework/Graphics/Audio/WaveformGraph.cs#L320)

音兰采用“原始分析与显示分辨率分离”的结构，同时保留峰值以免缩小后丢失瞬态。频带色彩计算增加零值保护；分析数据来自最终混音，以样本索引对应画面。

## 本次补充核读与落地（2026-10-03）

重新核读下方固定 revision 的 `audio_analyzer.py`：首尾分析窗口为 15 秒，`sort_tracks_optimized` 将目标曲线与首尾能量、BPM 关系用于候选评分；分析窗口、歌曲切换点和实际混音时间是不同概念。音兰在 `crates/audorchid-audio/src/arrangement.rs` 独立实现端点 FFT/立体声 RMS/起音包络自相关、可调能量曲线与确定性贪心排序，没有移植 Python/librosa 源码。配置顺序模式直接保留顺序，自动模式通过源索引同时驱动 PCM、封面和文字。

核读 GPUI `text_system/line.rs` 的 line metrics 基线与 cosmic glyph raster bounds：固定高度 mask 会裁到中英混排的下伸部，现按真实字形 bounds 校正位置，缓存栅格范围。加入读取实际 Roboto/Dream Han 的回归测试，确认旧布局确实会越界，并覆盖 gy/pq/j 与四种标题字号。

核读 OpenH264 0.9.8 `encoder.rs` 与原生 `ratectl.cpp`：RC_OFF 使用 spatial layer QP，不按这里传入的 iMinQp/iMaxQp 约束，不能用它实现当前 quality_qp 接口。最终采用 RC_QUALITY_MODE、相同的 QP 上下界和 High complexity；码率模式保留可选项。验证分别导出 QP 18 / 38 并解码比较，防止配置成为空操作。RGBA 转换保留显示 RGB 值，另用 256 级灰阶验证 limited-range 往返误差不超过 1。

## LiveSort 的分析与衔接

2026-10-03 按用户反馈重新逐段阅读同一固定提交的 `audio_analyzer.py` 82–144 行及 `main.py` 835–998、1118–1282 行。尾部分析在最多 40 秒的尾段上平滑 RMS，用相对峰值/底噪阈值估计无效尾部；`mix_entry_sec` 来自 `4 + invalid_tail_sec`，分析端限制 4～44 秒，导出端又限制为 2～14 秒，缺省回退为 8 秒。不能把首尾 15 秒的特征窗口当成固定交叠时长。[尾部分析](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/audio_analyzer.py#L82)、[交叠规划](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/main.py#L835)

其 `_get_transition_dynamic_profile` 根据首尾 RMS 差异修改 incoming 增益、outgoing 曲线和总增益约束；`_crossfade_state` 还控制入场延迟、duck、滤波深度、混响包络及入场后数秒的释放。`_apply_dynamic_fx` 对分块数据施加频域音色处理与卷积混响。这些都会让混入听感与单纯两秒淡化不同；长交叠也可能更容易听见两首同时参与。[动态包络](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/main.py#L946)、[效果实现](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/main.py#L1082)

早期该轮仅保留用户认可的画面及 2 秒等功率声音默认，只增加自行实现的 equal_power/smooth/linear 与形状控制。smooth 使用平滑时间相位后再做等功率淡化；没有照搬动态滤波、混响、自动尾部剪切或逐曲交叠推导。自动编排与手填顺序继续独立可选；另生成 4 秒 smooth 对照，帮助用户决定具体素材需要的融合程度。

重点读取 `LiveSortApp/audio_analyzer.py` 的尾部检测和歌曲排序（82–120、312–373 行），以及 `main.py` 的交叠规划、动态增益、包络和实际混音（835–853、946–998、1118–1269 行）。

分析器对首尾区域估计 BPM、RMS 能量和频谱亮度；尾部检测平滑 RMS、建立相对阈值，推导无效尾部与混入时间。排序将目标情绪、相邻 BPM 和能量差结合，并接受约二倍或一半 BPM 的相近关系。这是编排评分，不是实际 time-stretch 或保证拍点同步。[分析源码](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/audio_analyzer.py#L312)

导出端把重叠秒数转成样本数，并限制到两段可用数据长度。动态模式分别处理 outgoing/incoming 音色和混响包络，应用两组音量，再保留后续 PCM；最后按整段峰值缩放。音兰可以借鉴“先规划窗口、再生成连续包络、最后混音”的顺序。[实际混音](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/main.py#L1157)

源码虽然存在 sin/cos 等功率分支，但 `_build_transition_overlaps` 在 `automix = false` 时返回零交叠，因此正常导出调用中的该分支不能视为已验证启用的独立模式。动态模式还包含入场延迟和增益约束；音兰自行实现清楚的 gapless/crossfade 契约，不机械复制条件与常量。

原实现不断 `np.concatenate` 形成整段音频，且加载错误可能打印后跳过曲目。音兰长歌单采用块处理或磁盘缓存，声明的曲目解码失败默认终止。该仓库快照未找到项目级许可证文件，先参考算法思路，不把其代码纳入项目，许可问题另行解决。

## libyuv 的像素顺序与矩阵

读取 `include/libyuv/convert.h` 的接口说明、`source/convert.cc` 的 I420 入口与 SIMD 选择，以及转换头文件和许可证。

小端 ABGR 表示内存字节 RGBA；`ABGRToI420` 与 `RGBAToI420` 使用不同的通道常量。`ARGBToI420` 等便捷入口默认选 I601；`ARGBToI420Matrix` 才允许显式提供矩阵。音兰必须把 GPU 字节布局和色彩矩阵一起匹配，不能靠函数名猜测。[字节说明](https://github.com/lemenkov/libyuv/blob/eee459798486116575ed66c6d6609205d445ffb2/include/libyuv/convert.h#L986)、[矩阵与通道实现](https://github.com/lemenkov/libyuv/blob/eee459798486116575ed66c6d6609205d445ffb2/source/convert.cc#L3250)

转换按行处理并根据 CPU 能力、宽度对齐选择 SIMD。可借鉴这种参考标量、优化分支分离的结构。首版先验证 stride、2×2 色度降采样、上下方向和完整范围，之后决定是否引入 libyuv FFI。

## OpenH264 的真实输入与默认行为

读取 `src/formats/yuv.rs` 的 `YUVSource`、buffer 构造，`src/encoder.rs` 的配置、`encode_at`、VUI 与 bitstream 汇集，以及 `formats/arch/scalar.rs` 的转换。

当前 Rust encoder 明确构造 `videoFormatI420`，三平面分别提供指针和 stride；不能把 NV12 的 UV 交错平面套进它。`YUVBuffer` 要求宽高为偶数。`encode_at` 接收时间戳、尺寸改变会重新初始化。成片建议固定尺寸，减少码流与 MP4 sample description 的变化。[输入实现](https://docs.rs/crate/openh264/0.9.8/source/src/encoder.rs)、[平面契约](https://docs.rs/crate/openh264/0.9.8/source/src/formats/yuv.rs)

默认配置允许跳帧，且默认码率和帧率不是音兰的输出目标。必须显式配置，检查每帧输出和首关键帧。`write_vec`/`to_vec` 按层收集各个 NAL；一次编码的整体结果作为一个 access unit 交给 muxide。

内置标量转换使用 66/129/25 加 16 的 Y 系数及对应 UV 系数；按系数判断为 BT.601 limited 的近似路径。仅设置 BT.709 VUI 不会自动把这些转换系数变成 BT.709。音兰需要一致的转换与信令。[转换系数](https://docs.rs/crate/openh264/0.9.8/source/src/formats/arch/scalar.rs)

## wgpu 的映射生命周期

重点读取 `src/api/buffer.rs` 的 buffer CPU/GPU 互斥、映射、轮询与 unmap 文档，以及 GPUI 中的实际调用。

`map_async` 不是同步返回像素；回调需要 queue/device/instance 的进度驱动。映射状态下 buffer 不能参与 GPU 命令；必须释放映射 view 再复用。音兰采用有界 slot 和轮询驱动，而不是在每帧分配无限新 buffer 或期望回调自行触发。[buffer 源码](https://docs.rs/crate/wgpu/30.0.1/source/src/api/buffer.rs)

## muxide 的格式契约与扩展需求

读取 `src/api.rs` 的视频、音频写入和完成封装接口，`src/muxer/mp4.rs` 的 sample 存储、时间尺度、ADTS 处理与 finalization，以及 H.264/AAC 互操作示例。

H.264 输入是 Annex B access unit，AAC 输入是一帧完整 ADTS；writer 去掉 ADTS 后存储 raw AAC。首视频帧必须是关键帧并能提取 SPS/PPS，视频 PTS 要严格递增，音频 PTS 非负且不倒退。MP4 内部使用 90,000 ticks/s。[公共 API](https://docs.rs/crate/muxide/0.2.5/source/src/api.rs)

普通 `Mp4Writer` 的 `SampleInfo` 拥有 `Vec<u8>`，音视频 sample 留在 Vec 中到 finalize。standard 与 fast start 都不是恒定内存写入；相关路径限制 `mdat` 大小到 u32，并使用 32 位 chunk offset。首版仅以它验证短视频，长视频需选择磁盘缓存和 64 位扩展，或另验 fragmented 模式。[MP4 writer](https://docs.rs/crate/muxide/0.2.5/source/src/muxer/mp4.rs)

读取的普通 writer 没有发现 AAC priming/edit list 接口，不把“可以封装 AAC”当作“已经精确处理编码延迟”。现有互操作示例利用 ffmpeg 生成测试码流，也不能证明 fdk-aac 与 OpenH264 的音兰接入已通过。

## oxiaudio 的实际定位

读取 facade 的 decode/encode/DSP API，并追踪 `oxiaudio-decode` 的 PCM 拷贝、streaming 与 gapless 模块，以及 `oxiaudio-encode` 的 AAC 核心入口。

文件解码经 Symphonia，得到交错 `f32` PCM。streaming 路径用 FIFO 分块；存在 MP3 gapless 解析与裁剪工具，但调用方仍需确认解码路径是否已经应用，不能重复裁剪。[解码入口](https://docs.rs/crate/oxiaudio/0.2.1/source/src/decode.rs)、[gapless 工具](https://docs.rs/crate/oxiaudio-decode/0.2.1/source/src/gapless.rs)

当前版本确实也提供 AAC 编码，不应描述成只有 WAV/FLAC 编码。其 `encode_aac` 入口按 `len / frame_samples` 向下取整；这与不足帧尾部完整保留的目标需要进一步验证，注释不能替代测试。音兰保留用户指定的 fdk-aac 作为最终 AAC 后端，避免两个有损编码步骤。[AAC 实现](https://docs.rs/crate/oxiaudio-encode/0.2.1/source/src/aac.rs)

facade 有 STFT 与频谱分析入口，但本次未深入审计其 FFT 算法或进行音质基准。后续首先验证它能否覆盖窗口、时间戳和分析内存要求，再决定是否需要额外 FFT 依赖。

## fdk-aac 的适配问题

实际读取 `fdk-aac/src/enc.rs` 和 sys 包中的 `aac/libAACenc/include/aacenc_lib.h`。

Rust API 支持 AAC-LC、mono/stereo、ADTS/Raw transport，输入为 `&[i16]`，输出同时报告消费元素数和输出字节数。FDK InfoStruct 的 frameLength、nDelay 与 nDelayCore 能帮助管理帧边界和延迟。[Rust wrapper](https://docs.rs/crate/fdk-aac/0.8.0/source/src/enc.rs)

发现两处必须先验证和修补的边界：wrapper 把 `input_len` 填入 `bufSizes`，而 C 定义为字节数；wrapper 的长度由 slice 得到，不能传 `numInSamples = -1`，也未提供显式 flush。C 文档规定零样本只处理可能的重新初始化，EOF 需 -1 排空到结束。不得假定 `encode(&[])` 已排空编码器。[FDK 头文件](https://docs.rs/crate/fdk-aac-sys/0.5.0/source/aac/libAACenc/include/aacenc_lib.h)

这些是源码对照发现，尚未在本机复现编码输出。P0 用脉冲和非完整帧长度音频验证输入消费、最终样本数和音画对齐，再建立最小修补版。

## rubato 的尾块与延迟

读取 `src/lib.rs` 的 `Resampler` 契约、`process_into_buffer`、`process_all_into_buffer` 和 delay 处理。

5.0.1 使用 audioadapter 的 Adapter/AdapterMut，返回实际输入和输出样本帧数。最后不足块通过 `Indexing.partial_len` 表达，`output_delay` 以输出样本帧计。`process_all_into_buffer` 会处理 startup delay 和补零排空，说明自行写 streaming 适配也必须保留这些语义。[重采样实现](https://docs.rs/crate/rubato/5.0.1/source/src/lib.rs)

音兰采用固定采样率转换时保持一个 resampler 的连续状态，缓存可复用 buffer，记录实际长度。显示波形重采样与音频重采样是两种不同任务，不混用接口。

## 字体的实际检查

Roboto 阅读了 UFO 的 `pnum`/`tnum` 特性定义，并实际使用 FontTools 4.58.4 解析 `src/hinted/Roboto-Regular.ttf` 的 cmap、hmtx、GSUB 和 head。

| 检查 | 结果 |
| --- | --- |
| 文件 SHA256 | `56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d` |
| UPM | 2048 |
| 数字 0–9 默认 advance | 全部 1150 |
| 字体 GSUB 特性 | 包含 `tnum`、`pnum`、`lnum`、`onum` |

因此该具体文件的数字默认等宽；还需检查 GPUI feature 设置、字距、冒号和实际时长文本布局。不能推广到所有 Roboto 版本或称整个字体为等宽字体。[字体文件](https://github.com/googlefonts/roboto-2/blob/38062f4b4a0be4346d07a928408da21602545e9e/src/hinted/Roboto-Regular.ttf)、[特性源文件](https://github.com/googlefonts/roboto-2/blob/38062f4b4a0be4346d07a928408da21602545e9e/src/v2/Roboto-Regular.ufo/features.fea#L1029)

Dream Han 读取了 README、许可证和 `generate_config.py`。项目发布传统静态字重与区域版本；脚本把 SC/CN 映射到 zh-Hans 等语言标识。当前发布标签为 `dream-3.03-sans-2.005-serif-2.003`，API 记录发布时间为 2026-03-03 10:12:42 UTC，包含 CN 等地区包。没有下载数百 MB 的字体发布压缩包，因此尚未检验实际 face index、CJK 覆盖或渲染效果。[指定发布版本](https://github.com/Pal3love/dream-han-cjk/releases/tag/dream-3.03-sans-2.005-serif-2.003)

## 阅读形成的实现决定

1. 复用 GPUI 已有 2D 离屏 renderer，以小适配器接入完整视图、字体和错误传播。
2. 把播放器事件转换为离线计划，显式传入动画时间，删除成片中的操作控件。
3. PCM 解码、重采样、衔接和分析在前，AAC 只编码一次。
4. I420、Annex B、ADTS、每声道样本数、色彩矩阵与编码延迟都成为明确接口契约。
5. 先修补和验证 AAC 尾帧，再扩展长视频封装和 GPU 读回流水线。
6. 保留上游协议，按实际使用的代码和字体整理声明；Audorchid 的最终协议仍待选择。

## smart 衔接补齐

重新逐段阅读固定 revision 的 audio_analyzer.py 82–130、main.py 835–998、1043–1109、1118–1271。原实现的提前安静检测、排除无效尾部后的局部动态分析、混入推导、动态包络、音色/混响与原声释放，确实不能由统一综合 LUFS 或仅延长交叠替代。音兰现用独立 Rust 实现这些功能方向：50 ms 立体声功率与 0.55 秒平滑，持续尾部/前奏检测，局部 BS.1770 LUFS，有界补偿、样本插值包络、一阶频段交接、小型立体声扩散和两端原声连续。未复制 Python 代码或随机 IR/FFT 卷积实现，没有加入变速对拍。默认较温和的参数为产品选择，不宣称听感优于参考。
