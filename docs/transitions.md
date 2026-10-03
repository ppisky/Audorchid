# 衔接、局部响度与快速试听

日常只填歌曲即可：默认自动编排、逐曲 −16 LUFS 和 `smart` 衔接。编排选择顺序，衔接选择混入时间与处理方式，二者可独立控制。综合 LUFS 是整首的测量，常量增益不会消除原有的安静前奏、渐弱尾音或末尾静音。

## 三种模式

| 模式 | 行为 |
| --- | --- |
| smart（默认） | 检测低能量尾巴、已渐弱尾部和安静前奏；规划提前混入；局部 LUFS 匹配、有界能量补偿、频段交接、少量立体声扩散；平滑回到原声 |
| adaptive | 上一版仅按端点能量低谷搜索交叠长度，使用配置淡化曲线；不加入动态效果 |
| fixed | 严格按 audio.crossfade_seconds 与配置曲线淡化；默认固定值为 2 秒 |

`[audio.transitions] enabled = false` 是明确的关闭开关：直接拼接、不处理交叠效果，也没有黄色进度区域。`audio.crossfade_seconds = 0.0` 继续在所有模式下禁用交叠。非零值在 smart/adaptive 中表示启用自动规划，实际时长取决于 transitions；fixed 才按这个数字确定时长。需要原样播放无缝专辑时，选 configuration、crossfade_seconds=0，loudness off/program。

```toml
# 所有字段均可省略；以下仅说明覆盖方法。
[audio.transitions]
enabled = true # false：关闭所有融合与黄色区
mode = "smart"
min_seconds = 4.0
max_seconds = 24.0
analysis_seconds = 40.0
tail_threshold_db = -24.0
fade_threshold_db = -6.0
max_gain_db = 3.0
tone_strength = 0.35
reverb_mix = 0.08
release_seconds = 1.0
```

扫描范围最大 120 秒，必须 >= 最长交叠；最长交叠可设到 60 秒。尾部阈值 −60～−12 dB、渐弱阈值 −12～−1 dB 都相对于端点活跃能量，而非绝对 LUFS。max_gain_db 为 0～6 dB，tone_strength 为 0～1，reverb_mix 为 0～0.25，release_seconds 为 0.1～4 秒。音色/空间效果分别设 0 即关闭。不要把 transitions.max_gain_db（局部补偿）与 loudness.max_gain_db（整首归一化）混淆。

## 实际算法

1. 保留左右声道功率，构造 50 ms 包络，以约 0.55 秒平滑抑制单次鼓点。75 分位功率作为端点活跃参考。检测连续低能量尾巴与渐弱尾部；前奏检测要求持续进入而非单次脉冲。
2. 排除检测到的低能量尾巴/安静前奏，分别取最多八秒测局部 BS.1770 LUFS。匹配量限制在 max_gain_db 内；无有效测量时不自动增加匹配增益。
3. 给下一首的安静前奏、上一首低能量尾巴与短交接留出时间，并以渐弱起点建议更早入场。在 0.25 秒步长候选上比较预测半秒功率低谷，同时惩罚过长同时播放与过迟进入。短曲的单次上限为相邻曲长的一半，保证没有三曲同时重叠。
4. 自然尾音已经渐弱时提前推进下一首包络，不等待几何中点。局部匹配随接管平滑解除，额外功率补偿有上限，不无限放大静音。20 Hz 增益节点在每个 PCM 样本上插值，两端严格为原声/单位增益。
5. 在真正交叠范围内，用连续滤波状态给叠加的低频和明亮瞬态留空间，并加入温和四延迟立体声扩散。效果在两端平滑归零，接回原声不发生处理状态跳变。这是独立 Rust 实现，不是参考 Python 的随机噪声 IR/FFT 卷积移植。
6. 混音后进行整段响度测量与真峰值保护，可视化与连续 AAC 编码消费同一份最终 PCM。

原文件不改动，音频内容不自动裁掉、不变速、不对拍。高度相关/反相关信号可能偏离功率预测，最终真峰值检查仍按真实波形执行。尚未识别调性、乐句和人声切点，所以不能保证任意两首都产生音乐结构上的连贯。

`inspect.transitions` 报告 quiet_tail_seconds、fade_tail_seconds、intro_seconds、双方局部 LUFS、incoming_match_db、实际 overlap_seconds、entry_limited 和效果强度。entry_limited=true 表示提前入场所需空间超过配置/短曲上限，仍可能留下低谷；可增加扫描/交叠范围或手工配置。estimated_dip_db 是忽略相关性、滤波与混响的功率模型估计，不是输出 LUFS，更不是听感分数。实际输出响度见 loudness.output。

PCM 混音、封面渐变、章节与黄色进度都使用实际交叠长度。标题与计数在交叠中点切换，这仍是显示策略，不是局部能量相等时刻。进度展示当前歌曲，下一首切换时包含已经听到的前奏；不把两首合成一个进度条。

## 按衔接试听

```powershell
./target/release/audorchid transition-preview project.toml
./target/release/audorchid transition-preview project.toml --transition 2 --before 3 --after 8 -o output/second.wav
```

编号从 1 开始，按**实际编排后**的顺序：1 是第一首到第二首，2 是第二首到第三首。范围为真正交叠前 before 秒、整个交叠、交叠后 after 秒；默认各六秒，超出歌单范围自动截到首尾。CLI 会打印双方曲名、实际交叠长度与片段范围。省略 -o 时在配置的视频旁保存 `<视频名>.transition-01.wav`；不修改 TOML，不为片段重新归一化，也不需要 GPU。关闭衔接时仍能试听直接拼接的切换点；单首歌或越界编号明确报错。无交叠且 before/after 都为零会拒绝空片段。

## 任意时间范围试听

```powershell
./target/release/audorchid inspect project.toml
./target/release/audorchid audio-preview project.toml --seconds 120 --duration 30 -o output/listen.wav
```

试听使用最终编排后的样本时钟，保存实际准备好的立体声 16 位 PCM WAV，不为片段另行归一化，不需要 GPU。省略参数从开头取最多 15 秒；超出结尾的 duration 截到实际结尾；负数、非有限时间、零时长与覆盖输入拒绝，已有输出需 --overwrite。黄色区间可在 PNG preview 检查后再导出长视频。

## 参考核读与功能差异

LiveSort 的尾部扫描、无效尾部排除、局部动态能量、入场时机、动态增益、滤波/混响与释放确实是连在一起的。上一版只有交叠长度搜索，不能称为已实现这套适配。现在补齐这些功能方向，但使用自己的检测阈值、局部 LUFS、连续包络与轻量扩散；不声称是逐样本复刻或听感优于参考。[分析器 82–130](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/audio_analyzer.py#L82)、[动态包络 868–998](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/main.py#L868)、[效果与混音 1043–1271](https://github.com/LuN3cy/LiveSort/blob/787239aa90bc97ca0f5e7c2b5964f054e9e422d0/LiveSortApp/main.py#L1043)

参考代码中的 BPM 评分没有实际执行节拍同步/变速。ISMIR 2017 §4 将乐句切点与节拍同步渲染视为另外的处理层；音兰尚未实现它们，不能用这一点掩盖此前遗漏的尾部与动态处理。[论文](https://archives.ismir.net/ismir2017/paper/000086.pdf)

