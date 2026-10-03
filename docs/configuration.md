# 配置填写与固定版式

日常用[中文简洁版](../examples/audorchid.simple.zh-CN.toml)或[English simple](../examples/audorchid.simple.en.toml)，只写想覆盖的选项。查全部参数用[中文完整版](../examples/audorchid.zh-CN.toml)或[English complete](../examples/audorchid.en.toml)。两类示例分别保持中英文字段/值一致。TOML 内路径相对于配置文件；字段拼错仍会报错。

只有每首 [[tracks]] 的 audio 必填；title 省略/留空时用文件名，artist 默认空，cover 默认占位封面。schema_version 默认 1，output 默认 TOML 旁的 audorchid.mp4（不覆盖），fonts 默认内置 Roboto/Dream Han；都可明确覆盖。默认自动编排、逐曲 −16 LUFS、smart 自动衔接；audio.arrangement.enabled 和 audio.transitions.enabled 都默认 true，可分别明确关闭、白色进度、自适应白色频谱，见[默认值依据](defaults.md)。

仅改变 layout.preset 时自动选择标准尺寸；仅写 width 或 height 时按准确比例推导另一边，同时写两边则验证原值。标准尺寸为 1920×1080、1080×1920、1080×1080、1080×1440。临时缩小和改输出路径可用 CLI 参数。

## CLI 预览与临时导出参数

```powershell
# 默认取第 0 帧，PNG 路径由 output.path 派生
./target/release/audorchid preview examples/audorchid.zh-CN.toml
# 临时半尺寸取 4.2 秒处的帧
./target/release/audorchid preview examples/audorchid.zh-CN.toml --seconds 4.2 --scale 0.5 -o output/preview.png
# 临时导出名称；加 --scale 0.5 可生成半尺寸草稿
./target/release/audorchid render examples/audorchid.zh-CN.toml -o output/my-video.mp4
```

`preview` 不编码视频，默认保存第一张图；例如 output.path 为 `../output/audorchid.mp4`，默认 PNG 就是同目录 `audorchid.preview.png`。首帧的音频可视化可能还没有柱高，可用 `--seconds` 检查音乐开始之后的画面；时间必须在有效音频范围内，取不晚于指定时刻的视频帧。`frame` 保留为兼容别名。

`preview` 与 `render` 都支持 `-o/--output`、`--scale`、`--overwrite`。CLI 输出路径相对于终端当前目录，TOML 中的输入路径仍相对于配置文件，参数不修改 TOML。预览必须用 .png，视频必须用 .mp4。预览重复保存需加 `--overwrite`；视频也可以使用 TOML 的 `output.overwrite = true`。覆盖输入音频、封面、字体或配置文件会被拒绝。

`--scale` 范围为 0 < scale ≤ 1，默认 1。1920×1080 的 `--scale 0.5` 为 960×540；所有内置位置/字号和自定义矩形/字号同步缩放。实际尺寸取保持准确比例的最近偶数尺寸，命令报告实际分辨率；尺寸不能小于 32 或超出原尺寸，不可实现的缩放会报错。低分辨率适合检查布局，清晰度验收仍用 scale=1 的 PNG 或最终视频。

## 四种内置版式

内置 16:9、9:16、1:1、3:4。坐标为 `[左, 上, 宽, 高]`，单位为参考画布像素。同一比例的偶数分辨率会等比缩放全部位置和字号。

| 项目 | 16:9 · 1920×1080 | 9:16 · 1080×1920 | 1:1 · 1080×1080 | 3:4 · 1080×1440 |
| --- | --- | --- | --- | --- |
| 封面 | 130,170,620,620 | 120,240,840,840 | 300,80,480,480 | 220,120,640,640 |
| 标题 | 870,340,920,84 | 120,1140,840,77 | 90,610,900,58 | 90,812,900,65 |
| 作者 | 870,438,920,46 | 120,1230,840,43 | 90,678,900,36 | 90,888,900,39 |
| 底部波形/频谱 | 130,900,1660,130 | 120,1630,840,220 | 90,880,900,140 | 90,1190,900,190 |
| 作者下方小进度 | 870,588,772.8,4 | 120,1371.22,705.6,3.78 | 90,794.89,756,3.11 | 90,1013.67,756,3.33 |
| 时间 | 870,608,772.8,30 | 120,1390.11,705.6,28.33 | 90,810.44,756,23.33 | 90,1030.33,756,25 |
| 标题/作者/时间字号 | 70 / 36 / 22 | 64 / 34 / 20.78 | 48 / 28 / 17.11 | 54 / 30 / 18.33 |

上表为默认位置，带小数的值四舍五入到两位；精确坐标可用 `inspect` 查看。常规只选比例，不必填写这些坐标。

`layout.progress_position` 可选：

- `under_artist`：作者下方的小进度条，宽为文字区的 84%，横屏基准高度为 4 px；时间位于条下方。
- `under_artist_lower`（默认）：与 under_artist 的横向位置、宽度完全一致，只下移 72 × 作者字号 / 36 像素；横屏为 `[870,588,772.8,4]`。旧 `cover_side` 作为此位置的兼容别名，不再向封面横移。
- `attached`：进度条与频谱左边对齐，上边贴合频谱底边；时间位于频谱上方。
- `above`：进度和时间位于底部可视化上方。自定义版式使用手填的 progress/time 坐标。
- `auto`（省略时默认）：内置版式等同 under_artist_lower；自定义版式保持手填坐标。

`layout.visualizer_position` 默认 `bottom`，也可以选 `under_artist`，把频谱/波形移进作者下方的信息区。它与进度位置分别控制：二者都选 under_artist 时依次显示作者、小进度和时间、可视化；想让条贴着这块小频谱，使用 `visualizer_position = "under_artist"` 加 `progress_position = "attached"`。

可选 `visualizer_scale = [宽度倍率, 高度倍率]` 和 `progress_scale = [宽度倍率, 高度倍率]`，每项为 0.25～1.0，默认 `[1.0, 1.0]`。前者保持可视化左边和底边不动，后者保持进度左上角不动；时间行同步采用进度条宽度，计数可见右边缘与条末端对齐。需要放大到任意尺寸时使用 custom 坐标。位置选择先应用，缩放后检查是否落在画布内。

`progress_offset = [向右, 向下]` 可同时微调进度条和时间行，默认 `[0,0]`，负数为向左/向上；单位以画布短边 1080 像素为基准缩放。例如 `[0,24]` 保留原横向对齐，只再下移一点。最终矩形必须在画布内；手动偏移时需自行避免遮挡文字或可视化。

标题和作者参考原 Apple Music demo 的紧凑间距，内置标题单行显示，过长按真实字体宽度截断为 `...`；custom 的较高文字框仍可换行。数字启用 `tnum`、`lnum`，关闭 `pnum`。默认 `visual.progress_style.scope = "track"`：进度和时钟跟随当前歌曲。标题与计数在交叠中点同时切换，下一首的计时包含已经听到的淡入部分。选 `program` 才显示整段视频。黑色章节刻度已移除；黄色标出实际重叠混音的区间，而不是 15 秒分析窗口。16 秒 + 16 秒、交叠 2 秒时，第一首条末尾 14～16 秒标黄，第二首条开头 0～2 秒标黄；在视频第 15 秒切换标题时，第二首已播放 1 秒。没有交叠就没有黄色。封面在整个交叠区间渐变。`inspect.chapter_starts` 仍列出元数据切换的样本位置。

`[visual.progress_style]` 可调 `color`、`track_color`、`track_opacity`、`overlap_color`、`overlap_opacity`、`show_overlap`、`corner_radius`、`show_counter`、`counter_right_inset`。颜色为 `#RRGGBB`，透明度为 0～1；计数缩进默认 0，按可见笔画右边缘对齐进度条右端。字号与文字框可通过 custom 坐标设置。文字现在依据 GPUI 实际栅格字形边界校正基线，避免 CJK 与 Latin 混排时裁掉 g/y/p/q/j 的下伸部；自定义框小到容不下笔画时会明确报错。

进度默认 color=#FFFFFF，交叠提示仍为浅黄色，均可改色。`[visual.artwork_style] corner_radius = 24` 为稍圆的封面；0 为直角。border_width=3 保留浅色边框，0 关闭。background_blur=18 是 96×96 背景工作图的高斯 sigma（0～48），只影响背景。频谱 corner_radius=0 保留方块。圆角/边框/计数缩进按短边/1080 缩放，进度圆角限制为条高的一半。

## 可视化选择

**默认关闭三角形**：`visual.triangles = 0`。想开启时可填 48～72，最大 256。仅填写 triangle_style 不会自动开启。

`visual.triangle_style.preset` 提供两套参考 osu! 不同实现的独立预设：

- `layered`：大小和深浅变化的实心叠层；尺寸集中在 min_size/max_size 的中间，较大的移动更快；color/color_light 给出随机色端。
- `border`：固定 size 的等边三角形、不同上浮速度；color/color_light 给出画面上下的色端。thickness 是向内边框的相对厚度，0 不画边框，1 填满，默认 0.04。它使用归一化厚度，不受 stroke_width 控制。

`mode = "auto"` 跟随预设，layered 为实心、border 为边框；也可强制 filled/outline/both。layered 的空心模式使用 stroke_width 像素线宽。其它参数控制不透明度、颜色、速度和音乐响应；opacity 为 0 时完全透明，speed 为 0 时位置静止。颜色为 `#RRGGBB`；两个色端相同即可使用单色。尺寸与线宽以画布短边 1080 为基准缩放。所有三角形朝上，不随机旋转；seed 控制确定性的分布，支持任意时刻取帧。

例如先在已有 `[visual]` 中把 triangles 改为 60，再把已有 `[visual.triangle_style]` 的 preset 改为 border，mode 保持 auto 即可切换样式；无需重新配其它参数。

默认频谱是自适应白色宽柱、gap=0.30，不画峰值帽。柱数按实际区域宽度计算：底部预设 16:9 为 80、9:16 为 40、1:1/3:4 为 43，等比缩小视频不改变数量。以下是在原有表中修改的例子：

```toml
[visual.spectrum_style]
color = "#B9DCEC" # 浅蓝色；默认 #FFFFFF
bar_width = 14.5  # 自适应目标宽，短边 1080 为参考；越大柱子越少
# bars = 64       # 可选强制数量 8～256；填写后忽略 bar_width，不再自动减柱
gap = 0.30        # 空隙占每格的比例；越小柱子越粗/紧，越大空隙越宽
peak_caps = false # 可选同色峰值短线
```

这是对已有表的替换/修改，不能在同一个 TOML 再添加一个同名表。区域宽高用上面的 layout 缩放调整，通常保留预设即可。旧配置的 `palette = "white"` 仍固定为白色，改色时删掉它或改为 `solid`；新版示例无需填写 palette。此前的 `classic` 彩色样式已经删除，使用该值会明确报错。

自动柱数 = floor(参考区域宽度 × (1-gap) / bar_width)，限制 8～256；参考宽度为实际宽除以短边/1080。紧凑区域、自定义区域和 visualizer_scale 都参与计算；完整频率范围按实际柱数重新分箱。显式 bars 时，柱宽 = 区域宽度 / bars × (1-gap)，保持原手动语义；删掉 bars 恢复自动。bar_width 范围为 2～100 参考像素。

频谱固定 30 Hz～16 kHz 对数横轴、4096 点 Hann 窗；纵轴为相对 0 dBFS 的幅度（默认为 -60～0）。柱子和峰值帽的回落/保持时间可配，按音频 100 Hz 时钟预计算，与视频帧率无关。频率窗只读取当前时刻及以前的音频，约 43 ms 的窗中心延迟属于分析窗口；没有为了好看捏造频段能量。单频测试音只会亮起少数频段，这是正常的。

波形需将 `visual.waveform` 改为 true。`waveform_style.mode = "scope"` 是上下分开的左右声道示波线，直接显示最近 5～200 ms 带正负振幅的 PCM；window_ms 决定时间跨度，gain 只控制视觉高度。`"envelope"` 显示当前时刻前后各 6 秒的峰值包络，左亮右暗。频谱和波形同时开启时分区显示，不相互覆盖。

无有效音频、连续约 10 ms 内振幅低于 -100 dBFS 时，波形和频谱（包括峰值帽）隐藏。无声的包络柱不再有最小高度；音乐范围外不画占位点。歌曲中间的艺术性静音保留在音频和时间线上，此时封面/文字/装饰背景继续显示。

## LUFS 与音量

`[audio.loudness]` 默认 track / −16 / 12，不写也生效，显式 off 可关闭。−16 LUFS 是跨来源音乐的产品默认，依据见[默认值研究](defaults.md)，不是 EBU 广播的 −23 LUFS 目标。

| 模式 | 用途 | 手动增益关系 |
| --- | --- | --- |
| track | 不同来源歌曲先逐曲调整到相近响度 | 在归一化之后施加每首 gain_db，保留人工轻重差异 |
| program | 把完成的混音调整到目标响度 | 保留曲目之间的相对响度 |
| off | 不自动调整 LUFS | 仍测量响度，并执行最终真峰值保护 |

处理顺序是裁剪/重采样 → 逐曲 LUFS（track）→ 逐曲手动增益 → 可选自动编排 → 交叉淡化 → 整段 LUFS（program）→ 整体手动增益 → 真峰值保护。`max_gain_db` 只限制自动放大，不限制自动衰减。没有动态压缩器，使用整段常量增益保持动态和静音。

`audio.peak_ceiling_db` 现在是**编码前 dBTP 真峰值**上限，取代上一版的样本峰值判断；在 44.1/48 kHz 下采用 4 倍过采样。峰值保护优先于 LUFS，因此强瞬态素材可能低于目标响度。AAC 有损编码后峰值可能变化，此版本不宣称编码后绝对满足这个上限。

`inspect.loudness` 给出每首输入 LUFS/真峰值、自动增益、手动增益、混音调整前读数、最终读数和峰值衰减量。完全静音或不足 400 ms 时综合响度为 null，不自动放大；400 ms 以上也可能由于门限无法测量。`check`/导出日志会显示最终实际读数。算法采用 ebur128 的门限综合响度及真峰值测量；没有把 RMS 冒充 LUFS。[EBU R128](https://tech.ebu.ch/publications/r128)、[ebur128 API](https://docs.rs/ebur128/0.1.10/ebur128/struct.EbuR128.html)。

## 自定义位置

将原有 `[layout]` 段替换为以下内容；`[video]` 仍需指定实际输出宽高。自定义坐标直接使用输出像素，不套用内置比例。所有字段必须填写，矩形必须落在画布内。三角形和背景仍覆盖整张画布。

```toml
[layout]
preset = "custom"

[layout.custom]
cover = [130.0, 170.0, 620.0, 620.0]
title = [870.0, 340.0, 920.0, 84.0]
artist = [870.0, 438.0, 920.0, 46.0]
waveform = [130.0, 900.0, 1660.0, 130.0]
progress = [870.0, 588.0, 772.8, 4.0]
time = [870.0, 608.0, 772.8, 30.0]
title_size = 70.0
artist_size = 36.0
time_size = 22.0
```

以上示例对应 1920×1080。自定义时请保留足够的文字高度（行高约为字号的 1.2 倍），封面建议保持正方形。品牌小字位于封面上方，曲目序号位于时间区域右侧，不是可点击控件。

## 时间与衔接

`trim_start_seconds` 和 `trim_end_seconds` 都是源文件位置，先裁剪、再重采样。省略结束位置表示使用源文件剩余部分。时间转换为最近的源样本；重采样后的长度用整数比率向上取整。首版不自动去除艺术性静音。

`crossfade_seconds = 0.0` 将解码后的曲目直接拼接；非零时启用衔接：默认 smart 规划实际长度；fixed 才按该值及配置曲线执行（固定值默认 2 秒）。中间曲目长度必须大于等于两侧交叠之和。两首相似波形相加也可能增大峰值，最终混音必要时统一衰减到真峰值上限 `peak_ceiling_db`，CLI 会报告衰减量。

| audio 参数 | 默认值 | 含义 |
| --- | --- | --- |
| crossfade_seconds | 2.0 | 实际交叠秒数，0～30，必须容纳于曲长；与 15 秒分析窗口无关 |
| crossfade_curve | equal_power | cos/sin 等功率曲线；smooth 先平滑时间相位再应用等功率；linear 的两路增益之和为 1 |
| crossfade_shape | 1.0 | 0.25～4；1 保持原曲线，>1 将交接集中在中点，<1 更早参与混合 |

equal_power/smooth 的两路增益平方和为 1，适合不同歌曲；相同或高度相关的录音仍可能在中点叠加变响，linear 可减少这类叠加。smooth 起止更平缓，但不会自动选乐句、对拍或添加混响。试听更长融合可以在已有 `[audio]` 中改 `crossfade_seconds = 4.0`、`crossfade_curve = "smooth"`、`crossfade_shape = 1.0`；长交叠可能让两段旋律/鼓点相互干扰，应按素材调整。shape 对三种曲线均生效，不改变歌曲起点、封面过渡和黄色交叠区间；零交叠时忽略曲线。

所有曲目先形成一条连续 PCM 时间线，再经过同一个 AAC 编码器，切歌时不会重新启动编码器或插入等待。相同采样率无损 WAV 的任意样本切分实测：分成两段直接衔接后，解码出的 AAC 与整段音源导出的结果逐字节一致，见[衔接验收记录](research/gapless-validation.json)。

`[audio.arrangement] mode = "automatic"` 为默认，configuration 严格保留书写顺序。自动模式为独立实现的确定性贪心编排：裁剪/重采样、逐曲 LUFS 与增益后，读取首尾各 analysis_seconds（默认 15 秒，短曲取现有长度），测量 RMS、频谱质心与起音自相关节奏。energy_curve=[0.2,0.65,1.0,0.4] 引导起伏，综合首尾能量、亮度、节奏差选择下一首；四项 weight 可调。低置信度节奏为 null 并忽略，半拍/倍拍近似处理。keep_first=true 锁定开场；平分用原索引。过短曲目只放能容纳交叠的端点，不可行时报错。

`inspect.arrangement.order` 是实际播放顺序，元素为原配置从 0 开始的索引；`features` 和 `loudness.tracks` 按原配置顺序保留，时间线、封面、标题、计数、音频都按 order 执行。这是可解释的启发式排序，不保证全局最优或音乐上的最佳衔接。参考 LiveSort 的 15 秒首尾分析思路，没有复制其源码或宣称等同其完整编排器。

`[audio.arrangement] enabled = false` 强制按 TOML 顺序，不论 mode 是否 automatic；enabled=true 时才使用 mode（automatic/configuration）。`[audio.transitions] enabled = false` 优先关闭所有融合，无黄色区；enabled=true 时按 mode 与 crossfade_seconds 的兼容规则处理。

编排分析窗口不决定交叠时长。默认 smart 使用 40 秒首尾扫描，检测提前安静/淡出/前奏，在 4～24 秒范围规划交叠并处理局部增益、音色与空间尾音；所有参数都有默认值，可完全省略 transitions。固定秒数选 `[audio.transitions] mode = "fixed"`，上一版仅长度搜索选 `adaptive`。控制项及测量报告见[衔接文档](transitions.md)。实际长度仍受曲长限制，源素材保留，不对拍/变速；无缝专辑用 configuration、crossfade_seconds=0 和 loudness off/program。

省略 tracks.cover 时会自动生成圆环/渐变封面；可写 `cover_seed = 42` 固定变化。也可独立运行 `audorchid cover -o output/cover.png --size 1600 --seed 42` 生成 PNG，再填写 cover 路径。提供 PNG/JPEG 时按内容识别格式，不依赖下载文件的扩展名。封面原图不修改，裁切、缩放和圆角只用于画面。

视频帧数向上取整，末帧最多比有效音乐多保持不到一帧。AAC 尾帧全部排空，再用 MP4 edit list 表达起始编码延迟与有效音频长度。源有损格式的延迟处理由解码器负责，不重复手工扣除；跨格式无缝专辑建议先用无损素材验证。

## 导出清晰度

`[video] rate_control = "quality"` 为默认画质优先模式，`quality_qp = 18` 约束量化参数，0～51，越小细节越多且文件更大；这不是 CRF。`rate_control = "bitrate"` 使用 `bitrate_kbps` 的目标码率，quality_qp 在此模式不生效。quality 模式不承诺固定码率；视频仍为有损 H.264、BT.709 limited、I420 4:2:0，彩色细线会受色度采样限制。

保留 GPUI 输出的显示 RGB 数值进行 BT.709 矩阵转换，已移除原先造成视频中间调变暗的额外传递函数变换。文字按实际输出像素栅格化；封面以 Lanczos3 缩放到封面框尺寸，视频没有额外降分辨率。要检查清晰度可先用 preview 保存无压缩 PNG，再与同一帧的 MP4 解码图比较；观看时用原尺寸，缩小显示会再次重采样。可调整 width/height、quality_qp、bitrate_kbps、keyframe_interval；提高源封面分辨率也有助于保留细节。

## 当前边界

首版固定 SDR H.264 + 双声道 AAC-LC + MP4；44.1/48 kHz，1～120 fps，32～4096 的偶数宽高。字体文件的第一个 face 会被加载，缺字为错误。歌词、逐曲交叠参数、磁盘缓存和硬件编码尚未提供配置入口。

目前 PCM 与 muxide 压缩包保存在内存中；编码视频累计到 1.5 GB 会终止并清理临时文件。长歌单应先评估内存；流式 PCM 和增量 MP4 是后续工作。最终输出通过同目录临时文件完成后再发布，失败不替换已有输出。
