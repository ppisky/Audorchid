# 默认值与简洁配置

日常配置只要求至少一首 `[[tracks]]` 的 `audio` 路径。标题省略或留空时取不含扩展名的文件名；artist 默认空，cover 默认生成占位封面。schema_version 默认 1，输出默认 TOML 旁的 audorchid.mp4（不覆盖）；字体默认使用二进制内置的 Roboto/Dream Han，不依赖编译机器或配置文件的位置。每个明确填写的字体路径都必须存在。

| 项目 | 默认值 | 手动覆盖 |
| --- | --- | --- |
| 版式/尺寸 | 16:9、1920×1080、30 fps | layout.preset；video.width/height/fps_num/fps_den |
| 已播放进度 | 白色 #FFFFFF | visual.progress_style.color；位置/大小由 layout 控制 |
| 实际交叠提示 | 浅黄色 #EAC768 | overlap_color/overlap_opacity/show_overlap |
| 封面圆角 | 24 参考像素 | visual.artwork_style.corner_radius；0 为直角 |
| 频谱 | 自适应白色柱，目标宽 14.5、gap 0.3 | bar_width/gap；显式 bars 固定柱数 |
| 编排 | enabled=true、automatic，15 秒首尾窗口，舒缓→高点→收尾 | audio.arrangement.mode = configuration 恢复书写顺序 |
| 响度 | track、−16 LUFS，最多自动放大 12 dB | audio.loudness.mode/target_lufs/max_gain_db |
| 衔接 | enabled=true、smart，40 秒扫描，4～24 秒交叠，有界局部匹配与温和效果 | audio.transitions；fixed 才按 crossfade_seconds，0 始终禁用 |
| 音频输出 | 48 kHz、192 kb/s stereo AAC-LC、编码前 −1 dBTP 上限 | sample_rate/bitrate_kbps/peak_ceiling_db |
| 视频质量 | H.264 Quality、QP 18 | rate_control/quality_qp/bitrate_kbps |

只写 `[layout] preset = "3:4"`（分两行）即可得到 1080×1440；其它预设同样推导。仅写一个 width 或 height 时，另一边按精确比例推导并校验偶数尺寸；同时明确写两边则使用原值并校验。custom 的完整矩形/字号仍必须提供。CLI --scale 为临时缩小，-o 为临时输出名，均不修改 TOML。

## 频谱密度

auto 不再把 80 根柱塞进所有版式。以短边 1080 为参考，柱宽目标 14.5、gap=0.3，再按实际可视化区域的宽度取完整柱数（8～256）。默认底部区域：16:9 的 1660 px 为 80 根，9:16 的 840 px 为 40 根，方形/3:4 的 900 px 为 43 根。移动到作者下方、缩小区域、自定义矩形时也重新计算；仅等比缩小视频分辨率仍保持相同数量。

全部 30 Hz～16 kHz 频率在实际柱数上重新分箱，窄版式会合并频段，保持频率覆盖。明确写 `bars = 64` 就固定 64 根并忽略目标 bar_width；删掉 bars 恢复自适应。默认实际柱宽约 14.5～14.7 参考像素，避免竖版细挤。

## 阅读依据与选择

再次读取原 Apple Music demo 的封面容器与图像，二者都使用 `rounded_3xl`；音兰按自身 620 px 横屏封面将圆角从 18 小幅加到 24（约边长 3.9%），这是视觉选择，四比例预览验证后保留，而非音乐封面的统一标准。[原封面](https://github.com/cradiy/gpui-apple-music-demo/blob/ae4d5facbb29129c80b65e997440814a7e3b6e46/src/ui/components/now_playing.rs#L143)

实际阅读 ISMIR 2017《Automatic Playlist Sequencing and Transitions》的 §3、§4：顺序以音色、调性和节奏特征构建图，讨论贪心路径的局限；切点还需拍点、结构边界和人声因素。它支持按内容编排的方向，但未给所有曲风通用的最佳权重或固定秒数。音兰目前采用首尾能量/节奏/亮度的确定性贪心排序，不具备论文的神经音色嵌入、调性图、结构/人声切点模型。[论文](https://archives.ismir.net/ismir2017/paper/000086.pdf)

重新核对 LiveSort 的情绪曲线与排序：开场靠近目标能量，随后结合首尾 BPM、半拍/倍拍及能量差选择下一首。音兰将已存在的 automatic 变成默认；保持原顺序仍是一行覆盖，自动分析也不改变速度或裁剪源素材。[参考排序](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/audio_analyzer.py#L312)

DAFx 2016《Signal-Matched Power-Complementary Cross-Fading and Dry-Wet Mixing》§2 的混音功率包含两路相关性项：等功率适合不相关信号，相关信号可考虑线性；负相关补偿还有不稳定情形。音兰保留不同歌曲常用的 equal_power 默认，提供 linear/smooth 手调，没有声称已实现论文的相关性自适应曲线。fixed 模式保留 2 秒；日常默认 smart 根据实际端点决定长度和局部包络，不根据曲风盲目增加交叠。[论文](https://www.dafx.de/paper-archive/2016/dafxpapers/16-DAFx-16_paper_07-PN.pdf)

AES TD1008.1.21-9 的音乐归一化章节建议可行时采用专辑归一化，否则逐曲 −16 LUFS。音兰的跨来源歌单目前没有专辑分组，选择 track/−16；需要保留整张无缝专辑原始关系时明确选 configuration、crossfade=0、loudness off/program。program 是整段混音统一增益，不等同于该文献“以专辑最响歌曲为基准”的专辑归一化。峰值保护采用整体衰减，AAC 后仍可能出现不同峰值。[技术文档](https://aes.org/wp-content/uploads/2024/01/20210924_TD1008_v3.13.pdf)

## 默认行为变更

默认由 fixed 改为 smart，补齐早安静尾部识别、局部 LUFS 匹配、有界能量补偿、平滑频段与空间尾音交接。max_seconds=24、tone_strength=0.35、reverb_mix=0.08 是可调产品默认，未声称论文给出通用最佳值。明确配置 fixed/adaptive 的项目保留其模式，0 秒继续禁用。详见[衔接文档](transitions.md)。无封面也可用 cover CLI 导出原创圆环/渐变图。

本次省略 arrangement 时从 configuration 改为 automatic；省略 loudness 时从 off 改为 track。想保留先前明确的顺序/响度关系，就在对应表中填写 configuration/off。已有明确 bars 的配置仍固定柱数；旧的进度颜色和封面圆角也仍按显式值执行。

日常用 [中文简洁版](../examples/audorchid.simple.zh-CN.toml) / [English simple](../examples/audorchid.simple.en.toml)，查参数用 [中文完整版](../examples/audorchid.zh-CN.toml) / [English complete](../examples/audorchid.en.toml)。两类示例分别保持中英文数据一致；简洁版使用默认 smart；完整版为了演示原创短曲的一小节融合，明确用 fixed，所以两者混音长度不同。

开关明确写作 `[audio.arrangement] enabled = false` 与 `[audio.transitions] enabled = false`，分别强制原配置顺序、关闭融合及黄色区；不会互相影响。衔接 CLI 预览见 [transition-preview](transitions.md#按衔接试听)。
