# Vincentwei1021/video-talkcraft

- **Source:** https://github.com/Vincentwei1021/video-talkcraft
- **Kind:** github
- **Captured:** 2026-10-25T15:07:48
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** Vincentwei1021/video-talkcraft
- **Description:** Agent skill that turns Claude Code / Codex into a motion-design studio for voiceover-driven explainer videos — word-level voiceover sync, 109 motion recipe cards, an anti-slideshow camera system, Remotion rendering.
- **Homepage:** https://vincentwei1021.github.io/video-talkcraft/
- **Stars:** 1759
- **Forks:** 179
- **Open issues:** 1
- **Licence (SPDX):** NOASSERTION
- **Language:** HTML
- **Default branch:** main
- **Last push:** 2026-10-12T05:29:10Z
- **Archived:** False
- **Topics:** agent-skills, ai-agent, claude, claude-code, explainer-video, kinetic-typography, motion-design, motion-graphics, narration, remotion, talking-head, video-production

## README

```
<div align="center">

<img src="assets/logo.svg" alt="video-talkcraft logo" width="150">

<h1>video-talkcraft</h1>

[![Gallery](https://img.shields.io/badge/Gallery-live%20previews-7A5AF8)](https://vincentwei1021.github.io/video-talkcraft/)
[![License](https://img.shields.io/badge/License-PolyForm%20Noncommercial-blue)](LICENSE)
[![WeChat](https://img.shields.io/badge/WeChat-%E8%AE%A8%E8%AE%BA%E7%BE%A4-07C160?logo=wechat&logoColor=white)](assets/wechat-group.jpg)

**口播视频的 agent skill：字级配音同步 · 108 张动效配方卡 · 七层反 PPT 镜头系统 · 三重验收**

[中文](README.md) | [English](README_EN.md)

</div>

**video-talkcraft** 是 [video-shotcraft](https://github.com/Vincentwei1021/video-shotcraft)
系列的口播视频篇：一个把 Claude Code / Codex 变成口播视频动效工作室的 AI agent skill。
给它一份口播稿和一条成品配音，它在本机对齐字级时间戳、把每个语义拍写进 SHOTBOOK
分镜，然后用 [Remotion](https://www.remotion.dev/) 渲出高质量的解说成片——动态字卡、
证据截图、运镜、素排字幕、影视级音效，全部锁在人声上。

🖼️ [**在线画廊：108 张动效预览一页全览 »**](https://vincentwei1021.github.io/video-talkcraft/)

[![video-talkcraft 在线画廊](assets/gallery-zh.png)](https://vincentwei1021.github.io/video-talkcraft/)

## 统一依赖（runtime/）

所有口播工程的 `remotion/node_modules` 与 `workbench/node_modules` 都软链到 `runtime/node_modules`，版本只在 `runtime/package.json` 钉一处（`@remotion/*` 全家同号）。
- 新片开工：`bash runtime/check-runtime.sh --upgrade`（装依赖 + 共享无头浏览器 + 链 workbench；Remotion 有新版就升级并冒烟渲 1 帧，不过回滚）。
- 接工程：`bash runtime/link-runtime.sh <工程根>`（软链 + package.json 版本对齐）。工程目录里不跑 `npm install`。
- 一份 `node_modules` ≈ 760 MB（含 190 MB 无头浏览器）；之前每支片各装一份，六个工程 3.9 GB。

## 🆕 更新（What's new）
<!-- 写法约定：每条一句话、≤ 200 字（含指向链接），只写"是什么 + 改了什么行为"；细节、参数、卡名清单一律 → 指向 references / SKILL.md 章节，不在这里展开；中英同步。口径由 2026-09-05 PR #12 定，#14/#18 曾回到长段落，2026-09-07 收回。 -->

**2026-09-11**

- ✂️ **配音预剪 `scripts/voice_trim.py`**——真人录音先以口播稿为真值剪掉口水词 / 结巴重说 / 过长停顿再做时间戳（只剪稿子里没有的插入段，ASR 听错的字永不剪），词表来源可选 FireRed ASR / 逐字 SRT / 词级 JSON，切点吸附帧网格、同一 EDL 可剪人物视频保音画同长（→ SKILL.md ②-0）。
- 🫥 **工作台右键导出透明通道**——时间轨上右键任一动效片段可导出只含这一段的透明视频（MOV ProRes 4444 / WebM VP9 alpha），根底透明、自动去掉卡根层幕底与人物剪影占位，给剪映 / PR / AE 当叠加素材；右键菜单同时带分割 / 复制 / 删除（→ `workbench/GUIDE.md` ⑦）。
- 📺 **工作台实时看板（L1）**——⑤-1 骨架搭完就接入并开着工作台：阶段栏 + 进度轨（每镜 占位 / 已实现 / 已渲 / 已过闸 / 过期 / 红点）+ 镜头视图（单镜预览 · issues · SHOTBOOK 段落）+ 实时成片卡（2026-09-21 起单轨成片卡下线，看板即多轨），状态按盘上产物自动推、`scripts/pipeline_state.mjs --pass/--issue` 只补人工判定，代码半成品不再盖页（→ SKILL.md ⑤-2、`workbench/docs/live-pipeline.md`）。
- 🔗 **工作台接入加固**——`kbsrc` 改按真实路径解析、契约模块缺哪个只回退哪个 stub、导出形态差异由 `src/kb/` 适配层归一：接入 skill 正式工程（`Main.tsx` + `scenes/`，无 promo 模块）不再整页 500，「拆解导入」按钮按契约自动禁用（→ `workbench/README.md`「接入口播成片工程」）。

**2026-09-09**

- 🎬 **首镜先做先确认**——⑤ 实现改为先搭全合成骨架、其余镜头占位，只做样板镜并用 `--seg-audio` 出一条有声单镜预览给用户确认，确认后再做其余镜头；⑥-1.5 只剩"整片还是逐镜"一问（→ SKILL.md ⑤-1）。

**2026-09-07**

- ✏️ **G5 线稿示意图系统**——纯文字镜必配一张图标 + 方框 + 箭头的线稿陪衬图，文字不裸放、每句至少一个可见变化，缺陪衬行 preflight 报 WARN（→ `template/motion-systems/schematic.tsx`、`references/schematic.md`）。
- 🎞️ **视频容器边框八式 + 章节主题层**——单视频镜头必须包一式主题边框（浏览器窗 / 胶片 / 拍立得 / 图纸等八式，一片一式、不做假播放器），章节卡按章换色 + 线稿 motif（→ `template/components/theme-frame.tsx`、design-language §1.3）。

**2026-09-05**

- 🔁 **19 张 video-shotcraft 移植卡（89 → 108）**——从姊妹库 [video-shotcraft](https://github.com/Vincentwei1021/video-shotcraft) 两轮筛选移植（09-06 删定格圈注余 18），tsx 改成本库自包含契约、带母本溯源（→ `references/taxonomy.md` 第八批◎）。
- 🎨 **领域定风格 · 中性卡蒙皮**——开工先由口播稿判定领域、派生风格档写进 SHOTBOOK G0；库里的卡是中性 UI，进片按风格档改皮不改运动命门，每镜写蒙皮行（→ `references/design-language.md` §0）。
- 🧩 **10 张多素材同屏新卡（79 → 89）**——并列句排版、三联画接力、对比分屏、传送带、卡堆扇开、照片墙推轨 / 时间线照片带等，每卡标注输入类型与常用场景，选卡先按素材类型过滤（→ `references/taxonomy.md` 输入类型索引）。
- 📐 **排版规范 `references/layout.md`**——12 栏栅格、间距令牌、字阶最小档、包围盒不相交等九项自检，选中卡的「已知坑 / 落位自检」必须抄进 SHOTBOOK。
- 🎨 **12 款动态幕底入库**——`template/motion-systems/backdrop.tsx` 深浅各 6 款、frame 驱动零随机，默认幕底改为浅 `pastel-mesh-flow` / 深 `mesh-flow-dark`。

**2026-09-04**

- 🎥 **运动系统做减法**——每场景只保留一条极缓推拉相机曲线 + 让位，idle 微动 / 环境呼吸 / 脉冲全部默认关闭。
- 🌐 **网页拍摄不贴图**——网页素材不再静态贴屏，改为镜头滚动 / 巡游 / 放大镜 / 划重点"拍"出来，坐标全吃 Playwright 实测（→ SKILL.md §③、`references/shot-design.md` §2④）。

**2026-09-02**

- 🎛️ **动效工作台 `workbench/`**——剪映式的成片后期台：多轨时间线 + 素材库（素材 / 动效库 / 音效 / 背景）+
  schema 属性面板 + 实时预览 + 一键「导出成片」。108 张动效卡 100% 参数化（文案 / 颜色 / 字号 / 位置可调，
  节奏命门固定不暴露）；口播成片可一键拆成字幕 / 转场 / 环境 / 数字人 / 镜头 / 配音 / 音效七类多轨单元逐项微调。
  skill 交付成片后会主动打开它。→ [**图文指南 workbench/GUIDE.md**](workbench/GUIDE.md)

  <a href="workbench/GUIDE.md"><img src="workbench/docs/img/01-overview.png" alt="动效工作台总览" width="720"></a>

- ⚡ **渲染提速：分段渲染母版制**——`scripts/render_shots.mjs` 按镜头切段并行渲、段内单进程保光栅一致，
  拼装 + 整条音轨混入 + 帧数断言；改一个镜头只重渲该段±邻段。`scripts/render_stills.mjs` 一次 bundle 批量出静帧。
- 🧮 **评审 token 减量**——`scripts/contact_sheet.py` 把 QA 帧拼成 3×4 网格给评审子代理整版浏览；连拍三帧对只对标了
  `"burst": true` 的状态切换锚点抽（曾占评审材料 2/3）。
- ✅ **一轮审片即交付**——机器闸全过后只做 1 轮独立审片、修完 P0/P1 即交付，再询问是否续审（累计封顶 3 轮），
  替代旧的"循环到全过"。

| 实测（201s 竖屏片） | 之前 | 现在 |
|---|---|---|
| 全片首渲 | 13 min | 9 min |
| 改一个镜头出有声新片 | 整渲 | 53 s |
| 43 张静帧抽样 | 11 min | ~1 min |
| 评审读 160 张 QA 帧 | ≈16 万 token / 21 min | ≈4 万 token / 7 min（拼图） |

- 🤝 新卡：社区贡献 **douyin-follow-card 抖音主页关注卡**（[@scpcn01vision-oss](https://github.com/scpcn01vision-oss)），库存 79 张。

## ✨ 亮点

- **字级配音同步**——`scripts/timestamps_cpu.py` 把口播稿对齐到音频
  （默认 FireRedASR2-CTC int8，备选 faster-whisper 免手动下载）。110s 中英混合口播
  对照 GPU 强制对齐器实测：字级偏差中位 20–40ms、最差 200ms、质检零误报。
  每个动效节拍都锚在确切的字上。
- **108 张动效配方卡**——每张有意图、参数、已知坑、可直接复制的自包含 Remotion tsx 源码和可跑的 HTML 预览，
  [在线画廊](https://vincentwei1021.github.io/video-talkcraft/)一页全览
  （本地 `open gallery/index.html` 同款）。动态字卡、数据镜头、证据巡游、
  六式运动承接转场、长镜头世界画布、人物合成等。
- **七层反 PPT 系统**——每场景一条极缓推进 / 拉出的相机曲线 + 让位生命周期 + 六式运动承接转场
  （2026-09-04 起做减法：不再要求主体 idle 与环境呼吸）。
  静止帧在结构上不可能出现（漏网的也会被自动检测抓住）。
- **经得住审片的排版纪律**——语义拍分镜、同屏元素预算、留白锚、枢轴句切镜规则、
  用真实检测（`scripts/face_bbox.py`）量出来的人脸安全区，不靠目测。
- **三重验收**——画面健康双判定（静止段 + 并发光栅抖动，时域缺陷机器抓）、
  纯音效轨逐 cue 能量验证、带动效锚点帧与评审拼图的独立评审。

## 🚀 快速开始

**最直接的方式：把仓库链接丢给你的 agent。**
在 Claude Code / Codex 里直接说：

```text
帮我安装这个 skill：https://github.com/Vincentwei1021/video-talkcraft
```

或用 [skills](https://skills.sh/) CLI / 手动安装：

```bash
npx skills add Vincentwei1021/video-talkcraft
```

```bash
git clone https://github.com/Vincentwei1021/video-talkcraft.git
cd video-talkcraft
ln -s "$(pwd)" ~/.claude/skills/video-talkcraft   # Claude Code
# 或
ln -s "$(pwd)" ~/.codex/skills/video-talkcraft    # Codex
```

环境（agent 会按需自行配置）：

- Node 18+（Remotion 渲染；单片工程内 `npm install`）
- Python 3.10+
  - 本机 CPU 时间戳对齐：`pip install zhconv pypinyin sherpa-onnx soundfile numpy`
    （首次使用下载一次 767MB 的 FireRedASR2-CTC 模型，地址见
    `scripts/timestamps_cpu.py` 头注释；或加 `--backend whisper` 免手动下载）
  - 一键流式配音+时间戳（Fish Audio 免费层）：`pip install requests python-dotenv`
- ffmpeg

然后这样下需求：

```text
用 video-talkcraft 把这份口播稿 + voiceover.wav 做成视频。
做一条 100 秒的 <话题> 解说，稿子和音频在这里。
```

### 🎙️ 可选：Fish Audio 生成配音 + 字级时间戳

默认仍是「成品配音 + 本机 CPU 对齐」。没有录音时，可主动选择 Fish Audio 接入；脚本默认请求 `s2.1-pro-free`，模型的免费额度与可用期限以 Fish Audio 为准。

1. 安装 `ffmpeg` 和 Python 依赖 `pip install requests python-dotenv`。
2. 复制 `.env.example` 为 `.env`，填写 `FISH_AUDIO_API_KEY`。`FISH_AUDIO_REFERENCE_ID` 可填音色库中的音色 ID，留空使用服务默认音色。
3. 准备 `script.json`，例如 `{"sentences": ["这是第一句。", "这是第二句。"]}`；也支持字符串数组或每行一句的 `.txt`。
4. 生成音频与时间戳：
   ```bash
   # 默认逐句模式：每句一次请求，句间插入 0.25 秒真实静音
   python3 scripts/tts_fishaudio.py script.json audio/full.wav audio/timestamps.json --timing-out remotion/src/timing.json
   # 整稿一次请求，保留模型生成的句间节奏
   python3 scripts/tts_fishaudio.py script.json audio/full.wav audio/timestamps.json --timing-out remotion/src/timing.json --mode stream
   ```

两种模式均接收 SSE 音频与对齐快照；`stream` 表示流式接收，**文件在全部生成完成后写出，不提供边生成边播放**。`--pause-sec` 只控制逐句模式额外插入的静音。音频先分别解码成 PCM，再拼接、编码，时间戳按实际采样数累计。`--format mp3|wav|opus` 控制接口返回格式，输出格式由音频文件扩展名决定。

中文沿用接口的字级锚点；英文在词内按字符插值，标点零时长。空音频、缺失/无效对齐、解码失败、规范化后稿子与对齐文本不一致都会报错退出。规范化忽略标点、大小写与字符宽度，不推测 `2` 与 `two` 等读法转换；遇到此类差异请改写为实际朗读文本，或使用已有音频走本机 CPU 对齐。`match=1` / `ok=true` 表示文本映射通过，不代表已人工确认发音或字级精度，仍需试听。

回归验证（无需 API Key，需要 `requests` 与 `ffmpeg`）：
```bash
python3 -m unittest discover -s scripts -p 'test_*fish*.py' -v
```

## 🎞 你提供什么 vs. 它做什么

| 你提供（输入） | skill 负责 |
| --- | --- |
| 口播稿 | 字级时间戳对齐，逐句质检标记 |
| 成品配音——任何 TTS 或真人录音 | SHOTBOOK 分镜：语义拍、层矩阵、排版预算 |
| 可选的人物素材——普通实拍视频即可（抠像 + 人脸安全区工具已含，绿幕抠得最干净） | Remotion 实现：全局系统（极缓推拉相机/让位）、转场、音效落位 |
| 可选的 B-roll / 截图 | 渲染 + 三重验收（机器闸全过 + 一轮独立审片修完 P0/P1 即交付，可选续审累计 ≤3 轮），响度归一交付 |

## 📦 库里有什么

| 内容 | 说明 |
| --- | --- |
| 108 张动效配方卡 | 意图、能量档、参数、实现要点、已知坑——每张都配自包含 Remotion tsx 源码（`template/cards/`，复制单文件即用）+ 可跑的 HTML demo |
| 画廊 | [在线版](https://vincentwei1021.github.io/video-talkcraft/)或本地 `open gallery/index.html`——108 个预览一页自动播放，按名称/关键词搜索 |
| 动效系统 | CameraRig（极缓推拉）、让位生命周期、六式转场、长镜头世界画布；视差 / 环境层可选（`template/motion-systems/`） |
| 组件 | 素排字幕、花字、砸字、荧光笔、铅笔手绘、数字滚动、视频容器边框八式（`template/components/`） |
| 管线脚本 | 字级时间戳（双 ASR 后端）、人脸安全区检测、静止检测、音效在场检查、QA 抽帧（`scripts/`） |
| 方法论 | 设计语言（Apple 范式默认）、镜头三面工作单、电影感规范、分镜格式、验收口径（`references/`） |
| 内嵌音效 | 逐卡 cue 表 + 真采样内嵌 demo 库（授权见 `demos/_lib/sfx/ATTRIBUTION.md`） |

## 🗂 目录结构

```text
video-talkcraft/
├── SKILL.md                    # agent 入口：八步管线与硬规则
├── references/
│   ├── design-language.md      # 默认视觉系统（色板/字阶/布局/字幕）
│   ├── shot-design.md          # 三面工作单 + 七型镜头预设
│   ├── cinematography.md       # 七层模型、转场、排版预算、验收关卡
│   ├── shotbook-example.md     # 完整分镜范例
│   ├── cards/                  # 108 张动效配方卡
│   ├── taxonomy.md             # 按类别与来源的卡片索引
│   ├── broll-sources.md        # 免署名素材源（API、授权坑）
│   ├── host-footage.md         # 人物素材：输入规格、抠像、人脸安全区
│   └── demo-spec.md            # 卡片/demo 编写规范
├── demos/                      # 108 个可跑的 HTML 预览（共享库内嵌音效）
├── gallery/                    # 单页本地画廊
├── template/                   # 即取即用的 Remotion 代码
│   ├── cards/                  # 108 卡逐卡自包含 tsx 源码（skill 首选引用）
│   ├── motion-systems/         # 相机/视差/让位/环境/转场/长镜头系统
│   └── components/             # 字幕/花字/砸字/铅笔等组件
└── scripts/                    # 时间戳、人脸检测、QA 工具
```

完整工作流从 [SKILL.md](SKILL.md) 进入。

## ❓ FAQ

**video-talkcraft 是什么？**
一个开源的 AI agent skill（Claude Code / Codex 技能包），用于 AI 视频制作：
把口播稿 + 成品配音自动做成带动效的口播视频。它不是剪辑软件，也不是模板站——
agent 读方法论、选动效配方卡、写 [Remotion](https://www.remotion.dev/) 代码、
跑三重验收，产出可直接发布的解说成片。

**能做哪类视频？**
知识科普、产品评测、新闻解读、观点锐评等口播/解说类横屏视频。
中文口播优先设计，中英混排完全支持。

**需要准备什么？**
口播稿（文本）+ 成品配音（任何 TTS 或真人录音）；人物出镜素材与 B-roll 可选。

**免费吗？**
个人、教育、研究用途免费（PolyForm Noncommercial 1.0.0），
用它做出的视频归你所有；工具本身的商业使用需先授权（见下）。

## 📄 许可

[PolyForm Noncommercial 1.0.0](LICENSE)——个人、教育、研究用途免费。
**将本工具用于任何商业用途需事先获得授权**——发邮件至
[vincentwei1021@gmail.com](mailto:vincentwei1021@gmail.com) 或提 GitHub issue 联系。

**用本 skill 做出的视频归你所有。** 如果它帮到了你，欢迎在视频简介里
@ 一下作者的账号——非强制，但对作者是最好的支持。

## 🔊 音频与素材说明

- 内嵌音效采样的来源与授权：[demos/_lib/sfx/ATTRIBUTION.md](demos/_lib/sfx/ATTRIBUTION.md)。
- B-roll 素材源指南只收免署名源（Pexels、Pixabay、Mixkit Free、Coverr、NASA），
  并记录了被排除源的授权陷阱——见 [references/broll-sources.md](references/broll-sources.md)。
- demo 里的主持人素材（`demos/_lib/dh-host.webm`）是 AI 生成的演示形象占位，
  生产时请替换为你自己的人物素材。

## 🙏 致谢

- **[Remotion](https://www.remotion.dev/)**——驱动全部渲染的 React 视频框架
  （注意其自身[许可](https://github.com/remotion-dev/remotion/blob/main/LICENSE.md)）。
- **[FireRedASR2](https://github.com/FireRedTeam/FireRedASR2S)**（经
  **[sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)**）与
  **[faster-whisper](https://github.com/SYSTRAN/faster-whisper)**——时间戳后端；
  **Qwen3-ASR/ForcedAligner** 是精度基准参照。
- **OpenCV YuNet**——人脸安全区规则背后的检测器。
- **Pexels · Pixabay · NASA · Mixkit**——免署名素材来源。
- **Claude Code**——本库由 AI 编码 agent 构建、迭代与验收，用的正是 skill 自己教的那套评审循环。

## 关注作者

<p>
  <a href="https://www.douyin.com/user/MS4wLjABAAAAK1pkjBxilk2Oi_9h_vFyD-lTAu9CTlvhmOtkosDvvxg"><img alt="在抖音关注作者" src="https://img.shields.io/badge/%E6%8A%96%E9%9F%B3-%E5%85%B3%E6%B3%A8%E6%88%91-000000?style=for-the-badge&logo=tiktok&logoColor=white"></a>
  <a href="https://xhslink.cn/m/At9iP2d5C1V"><img alt="在小红书关注作者" src="https://img.shields.io/badge/%E5%B0%8F%E7%BA%A2%E4%B9%A6-%E5%85%B3%E6%B3%A8%E6%88%91-FF2442?style=for-the-badge&logo=xiaohongshu&logoColor=white"></a>
  <a href="https://x.com/VincentWei93"><img alt="在 X 关注作者" src="https://img.shields.io/badge/X-%E5%85%B3%E6%B3%A8%E6%88%91-000000?style=for-the-badge&logo=x&logoColor=white"></a>
</p>

## 微信讨论群

有建议、反馈或使用问题？扫码加入 video-talkcraft 交流群（3 群）：

<img src="assets/wechat-group.jpg" alt="video-talkcraft 微信交流群二维码" width="300">

二维码更新于 2026-10-12，2026-10-19 前有效；过期后会不定期更新，也可通过上方社媒直接联系作者。
```

## Licence file

```
video-talkcraft

Required Notice: Copyright (c) 2026 Vincent Wei (https://github.com/Vincentwei1021/video-talkcraft)

Videos produced with this toolkit belong to their creators. This license
governs the toolkit itself: noncommercial use is free; any commercial use
of the toolkit requires prior authorization from the author.

---

# PolyForm Noncommercial License 1.0.0

<https://polyformproject.org/licenses/noncommercial/1.0.0>

## Acceptance

In order to get any license under these terms, you must agree
to them as both strict obligations and conditions to all
your licenses.

## Copyright License

The licensor grants you a copyright license for the
software to do everything you might do with the software
that would otherwise infringe the licensor's copyright
in it for any permitted purpose.  However, you may
only distribute the software according to [Distribution
License](#distribution-license) and make changes or new works
based on the software according to [Changes and New Works
License](#changes-and-new-works-license).

## Distribution License

The licensor grants you an additional copyright license
to distribute copies of the software.  Your license
to distribute covers distributing the software with
changes and new works permitted by [Changes and New Works
License](#changes-and-new-works-license).

## Notices

You must ensure that anyone who gets a copy of any part of
the software from you also gets a copy of these terms or the
URL for them above, as well as copies of any plain-text lines
beginning with `Required Notice:` that the licensor provided
with the software.  For example:

> Required Notice: Copyright Yoyodyne, Inc. (http://example.com)

## Changes and New Works License

The licensor grants you an additional copyright license to
make changes and new works based on the software for any
permitted purpose.

## Patent License

The licensor grants you a patent license for the software that
covers patent claims the licensor can license, or becomes able
to license, that you would infringe by using the software.

## Noncommercial Purposes

Any noncommercial purpose is a permitted purpose.

## Personal Uses

Personal use for research, experiment, and testing for
the benefit of public knowledge, personal study, private
entertainment, hobby projects, amateur pursuits, or religious
observance, without any anticipated commercial application,
is use for a permitted purpose.

## Noncommercial Organizations

Use by any charitable organization, educational institution,
public research organization, public safety or health
organization, environmental protection organization,
or government institution is use for a permitted purpose
regardless of the source of funding or obligations resulting
from the funding.

## Fair Use

You may have "fair use" rights for the software under the
law. These terms do not limit them.

## No Other Rights

These terms do not allow you to sublicense or transfer any of
your licenses to anyone else, or prevent the licensor from
granting licenses to anyone else.  These terms do not imply
any other licenses.

## Patent Defense

If you make any written claim that the software infringes or
contributes to infringement of any patent, your patent license
for the software granted under these terms ends immediately. If
your company makes such a claim, your patent license ends
immediately for work on behalf of your company.

## Violations

The first time you are notified in writing that you have
violated any of these terms, or done anything with the software
not covered by your licenses, your licenses can nonetheless
continue if you come into full compliance with these terms,
and take practical steps to correct past violations, within
32 days of receiving notice.  Otherwise, all your licenses
end immediately.

## No Liability

***As far as the law allows, the software comes as is, without
any warranty or condition, and the licensor will not be liable
to you for any damages arising out of these terms or the use
or nature of the software, under any kind of legal claim.***

## Definitions

The **licensor** is the individual or entity offering these
terms, and the **software** is the software the licensor makes
available under these terms.

**You** refers to the individual or entity agreeing to these
terms.

**Your company** is any legal entity, sole proprietorship,
or other kind of organization that you work for, plus all
organizations that have control over, are under the control of,
or are under common control with that organization.  **Control**
means ownership of substantially all the assets of an entity,
or the power to direct its management and policies by vote,
contract, or otherwise.  Control can be direct or indirect.

**Your licenses** are all the licenses granted to you for the
software under these terms.

**Use** means anything you do with the software requiring one
of your licenses.
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (928 files, filtered)

- `LICENSE`
- `README.md`
- `README_EN.md`
- `SKILL.md`
- `THIRD_PARTY_NOTICES.md`
- `demos/_lib/demo-shell.js`
- `demos/_lib/gsap.min.js`
- `demos/_lib/lottie.min.js`
- `demos/_lib/media/ATTRIBUTION.md`
- `demos/_lib/sfx-map.js`
- `demos/_lib/sfx-samples.js`
- `demos/_lib/sfx.js`
- `demos/_lib/sfx/ATTRIBUTION.md`
- `demos/alt-block-lines/index.html`
- `demos/bar-chart-growth/index.html`
- `demos/bed-echo-blur/index.html`
- `demos/behind-text-title/index.html`
- `demos/black-slam-transition/index.html`
- `demos/callout-line-label/index.html`
- `demos/caret-wipe-transition/index.html`
- `demos/chapter-progress-list/index.html`
- `demos/chapter-title-card/index.html`
- `demos/chart-grow/index.html`
- `demos/chat-gpt/index.html`
- `demos/chat-message-flow/index.html`
- `demos/chevron-lower-third/index.html`
- `demos/chip-grid-single-select/index.html`
- `demos/claude-code/index.html`
- `demos/color-slam-beat-card/index.html`
- `demos/converging-arrows/index.html`
- `demos/corner-bracket-frame/index.html`
- `demos/count-badge-title/index.html`
- `demos/countdown-arc-scatter/index.html`
- `demos/crash-zoom-punch/index.html`
- `demos/cursor-actor-demo/index.html`
- `demos/cursor-locked-zoom/index.html`
- `demos/danmu-bubble-praise/index.html`
- `demos/doc-park-left-pill-deal/index.html`
- `demos/douyin-follow-card/index.html`
- `demos/error-retype/index.html`
- `demos/evidence-scroll-tour/index.html`
- `demos/filmstrip-conveyor/index.html`
- `demos/flying-words/index.html`
- `demos/focus-dim-spotlight/index.html`
- `demos/gallery-wall-dolly/index.html`
- `demos/glass-code-walk/index.html`
- `demos/gooey-morph/index.html`
- `demos/grid-to-hero/index.html`
- `demos/hand-drawn-ellipse/index.html`
- `demos/highlighter-sweep/index.html`
- `demos/host-card-glass-board/index.html`
- `demos/host-shrink-to-chip/index.html`
- `demos/impact-open-title/index.html`
- `demos/info-card-assemble/index.html`
- `demos/info-term-card/index.html`
- `demos/ink-underline/index.html`
- `demos/keyword-pop-highlight/index.html`
- `demos/lead-word-zoom-assemble/index.html`
- `demos/line-by-line-slide/index.html`
- `demos/line-carry-transition/index.html`
- `demos/line-chart-story-draw/index.html`
- `demos/logo-enter/index.html`
- `demos/long-take-world/index.html`
- `demos/lower-third-nameplate/index.html`
- `demos/magnifier-detail/index.html`
- `demos/map-route-pin/index.html`
- `demos/media-pop-in/index.html`
- `demos/metric-with-sparkline/index.html`
- `demos/motion-blur-slam-in/index.html`
- `demos/news-card-desk/index.html`
- `demos/number-counter/index.html`
- `demos/number-slab-pop/index.html`
- `demos/numbered-step-stack/index.html`
- `demos/orbit-drift/index.html`
- `demos/outline-box-title/index.html`
- `demos/overexpose-flip-transition/index.html`
- `demos/parallel-items-with-host/index.html`
- `demos/particle-weld-transition/index.html`
- `demos/pencil-sketch-draw/index.html`
- `demos/per-character-rise/index.html`
- `demos/pip-zoom-box/index.html`
- `demos/pullback-cool-transition/index.html`
- `demos/push-through-transition/index.html`
- `demos/quote-bracket-pull/index.html`
- `demos/quote-card/index.html`
- `demos/quote-hold-arrow/index.html`
- `demos/rack-focus-pair/index.html`
- `demos/reticle-lock-on/index.html`
- `demos/scanline-annotate/index.html`
- `demos/scribble-annotation/index.html`
- `demos/shape-wipe-transition/index.html`
- `demos/slab-punch-title/index.html`
- `demos/slow-pull-reveal/index.html`
- `demos/slow-push-in/index.html`
- `demos/soft-blur-in/index.html`
- `demos/source-converge/index.html`
- `demos/speed-slab-title/index.html`
- `demos/split-60-40-story/index.html`
- `demos/split-compare-slider/index.html`
- `demos/split-text-stagger/index.html`
- `demos/stack-fan-out/index.html`
- `demos/stage-keyframe-tour/index.html`
- `demos/step-timeline-vertical/index.html`
- `demos/still-layout-relay/index.html`
- `demos/strike-and-replace/index.html`
- `demos/subscribe-cta/index.html`
- `demos/sway-parallax/index.html`
- `demos/terminal-typing-log/index.html`
- `demos/tilt-3d-page/index.html`
- `demos/timeline-photo-strip/index.html`
- `demos/title-demote-to-label/index.html`
- `demos/tracking-in/index.html`
- `demos/type-contrast-emphasis/index.html`
- `demos/typewriter-reveal/index.html`
- `demos/ui-flow-theater/index.html`
- `demos/ui-prop-theater/index.html`
- `demos/unit-grid-proportion/index.html`
- `demos/whip-pan-transition/index.html`
- `demos/word-relay-filmstrip/index.html`
- `demos/word-slot-cycle/index.html`
- `demos/x-follow-card/index.html`
- `gallery/i18n/en/alt-block-lines.md`
- `gallery/i18n/en/bar-chart-growth.md`
- `gallery/i18n/en/bed-echo-blur.md`
- `gallery/i18n/en/behind-text-title.md`
- `gallery/i18n/en/black-slam-transition.md`
- `gallery/i18n/en/callout-line-label.md`
- `gallery/i18n/en/caret-wipe-transition.md`
- `gallery/i18n/en/chapter-progress-list.md`
- `gallery/i18n/en/chapter-title-card.md`
- `gallery/i18n/en/chart-grow.md`
- `gallery/i18n/en/chat-gpt.md`
- `gallery/i18n/en/chat-message-flow.md`
- `gallery/i18n/en/chevron-lower-third.md`
- `gallery/i18n/en/chip-grid-single-select.md`
- `gallery/i18n/en/claude-code.md`
- `gallery/i18n/en/color-slam-beat-card.md`
- `gallery/i18n/en/converging-arrows.md`
- `gallery/i18n/en/corner-bracket-frame.md`
- `gallery/i18n/en/count-badge-title.md`
- `gallery/i18n/en/countdown-arc-scatter.md`
- `gallery/i18n/en/crash-zoom-punch.md`
- `gallery/i18n/en/cursor-actor-demo.md`
- `gallery/i18n/en/cursor-locked-zoom.md`
- `gallery/i18n/en/danmu-bubble-praise.md`
- `gallery/i18n/en/doc-park-left-pill-deal.md`
- `gallery/i18n/en/douyin-follow-card.md`
- `gallery/i18n/en/error-retype.md`
- `gallery/i18n/en/evidence-scroll-tour.md`
- `gallery/i18n/en/filmstrip-conveyor.md`
- `gallery/i18n/en/flying-words.md`
- `gallery/i18n/en/focus-dim-spotlight.md`
- `gallery/i18n/en/gallery-wall-dolly.md`
- `gallery/i18n/en/glass-code-walk.md`
- `gallery/i18n/en/gooey-morph.md`
- `gallery/i18n/en/grid-to-hero.md`
- `gallery/i18n/en/hand-drawn-ellipse.md`
- `gallery/i18n/en/highlighter-sweep.md`
- `gallery/i18n/en/host-card-glass-board.md`
- `gallery/i18n/en/host-shrink-to-chip.md`
- `gallery/i18n/en/impact-open-title.md`
- `gallery/i18n/en/info-card-assemble.md`
- `gallery/i18n/en/info-term-card.md`
- `gallery/i18n/en/ink-underline.md`
- `gallery/i18n/en/keyword-pop-highlight.md`
- `gallery/i18n/en/lead-word-zoom-assemble.md`
- `gallery/i18n/en/line-by-line-slide.md`
- `gallery/i18n/en/line-carry-transition.md`
- `gallery/i18n/en/line-chart-story-draw.md`
- `gallery/i18n/en/logo-enter.md`
- `gallery/i18n/en/long-take-world.md`
- `gallery/i18n/en/lower-third-nameplate.md`
- `gallery/i18n/en/magnifier-detail.md`
- `gallery/i18n/en/map-route-pin.md`
- `gallery/i18n/en/media-pop-in.md`
- `gallery/i18n/en/metric-with-sparkline.md`
- `gallery/i18n/en/motion-blur-slam-in.md`
- `gallery/i18n/en/news-card-desk.md`
- `gallery/i18n/en/number-counter.md`
- `gallery/i18n/en/number-slab-pop.md`
- `gallery/i18n/en/numbered-step-stack.md`
- `gallery/i18n/en/orbit-drift.md`
- `gallery/i18n/en/outline-box-title.md`
- `gallery/i18n/en/overexpose-flip-transition.md`
- `gallery/i18n/en/parallel-items-with-host.md`
- `gallery/i18n/en/particle-weld-transition.md`
- `gallery/i18n/en/pencil-sketch-draw.md`
- `gallery/i18n/en/per-character-rise.md`
- `gallery/i18n/en/pip-zoom-box.md`
- `gallery/i18n/en/pullback-cool-transition.md`
- `gallery/i18n/en/push-through-transition.md`
- `gallery/i18n/en/quote-bracket-pull.md`
- `gallery/i18n/en/quote-card.md`
- `gallery/i18n/en/quote-hold-arrow.md`
- `gallery/i18n/en/rack-focus-pair.md`
- `gallery/i18n/en/reticle-lock-on.md`
- `gallery/i18n/en/scanline-annotate.md`
- `gallery/i18n/en/scribble-annotation.md`
- `gallery/i18n/en/shape-wipe-transition.md`
- `gallery/i18n/en/slab-punch-title.md`
- `gallery/i18n/en/slow-pull-reveal.md`
- `gallery/i18n/en/slow-push-in.md`
- `gallery/i18n/en/soft-blur-in.md`
- `gallery/i18n/en/source-converge.md`
- `gallery/i18n/en/speed-slab-title.md`
- `gallery/i18n/en/split-60-40-story.md`
- `gallery/i18n/en/split-compare-slider.md`
- `gallery/i18n/en/split-text-stagger.md`
- `gallery/i18n/en/stack-fan-out.md`
- `gallery/i18n/en/stage-keyframe-tour.md`
- `gallery/i18n/en/step-timeline-vertical.md`
- `gallery/i18n/en/still-layout-relay.md`
- `gallery/i18n/en/strike-and-replace.md`
- `gallery/i18n/en/subscribe-cta.md`
- `gallery/i18n/en/sway-parallax.md`
- `gallery/i18n/en/terminal-typing-log.md`
- `gallery/i18n/en/tilt-3d-page.md`
- `gallery/i18n/en/timeline-photo-strip.md`
- `gallery/i18n/en/title-demote-to-label.md`
- `gallery/i18n/en/tracking-in.md`
- `gallery/i18n/en/type-contrast-emphasis.md`
- `gallery/i18n/en/typewriter-reveal.md`
- `gallery/i18n/en/ui-flow-theater.md`
- `gallery/i18n/en/ui-prop-theater.md`
- `gallery/i18n/en/unit-grid-proportion.md`
- `gallery/i18n/en/whip-pan-transition.md`
- `gallery/i18n/en/word-relay-filmstrip.md`
- `gallery/i18n/en/word-slot-cycle.md`
- `gallery/i18n/en/x-follow-card.md`
- `gallery/index.html`
- `references/broll-sources.md`
- `references/cards/alt-block-lines.md`
- `references/cards/bar-chart-growth.md`
- `references/cards/bed-echo-blur.md`
- `references/cards/behind-text-title.md`
- `references/cards/black-slam-transition.md`
- `references/cards/callout-line-label.md`
- `references/cards/caret-wipe-transition.md`
- `references/cards/chapter-progress-list.md`
- `references/cards/chapter-title-card.md`
- `references/cards/chart-grow.md`
- `references/cards/chat-gpt.md`
- `references/cards/chat-message-flow.md`
- `references/cards/chevron-lower-third.md`
- `references/cards/chip-grid-single-select.md`
- `references/cards/claude-code.md`
- `references/cards/color-slam-beat-card.md`
- `references/cards/converging-arrows.md`
- `references/cards/corner-bracket-frame.md`
- `references/cards/count-badge-title.md`
- `references/cards/countdown-arc-scatter.md`
- `references/cards/crash-zoom-punch.md`
- `references/cards/cursor-actor-demo.md`
- `references/cards/cursor-locked-zoom.md`
- `references/cards/danmu-bubble-praise.md`
- `references/cards/doc-park-left-pill-deal.md`
- `references/cards/douyin-follow-card.md`
- `references/cards/error-retype.md`
- `references/cards/evidence-scroll-tour.md`
- `references/cards/filmstrip-conveyor.md`
- `references/cards/flying-words.md`
- `references/cards/focus-dim-spotlight.md`
- `references/cards/gallery-wall-dolly.md`
- `references/cards/glass-code-walk.md`
- `references/cards/gooey-morph.md`
- `references/cards/grid-to-hero.md`
- `references/cards/hand-drawn-ellipse.md`
- `references/cards/highlighter-sweep.md`
- `references/cards/host-card-glass-board.md`
- `references/cards/host-shrink-to-chip.md`
- `references/cards/impact-open-title.md`
- `references/cards/info-card-assemble.md`
- `references/cards/info-term-card.md`
- `references/cards/ink-underline.md`
- `references/cards/keyword-pop-highlight.md`
- `references/cards/lead-word-zoom-assemble.md`
- `references/cards/line-by-line-slide.md`
- `references/cards/line-carry-transition.md`
- `references/cards/line-chart-story-draw.md`
- `references/cards/logo-enter.md`
- `references/cards/long-take-world.md`
- `references/cards/lower-third-nameplate.md`
- `references/cards/magnifier-detail.md`
- `references/cards/map-route-pin.md`
- `references/cards/media-pop-in.md`
- `references/cards/metric-with-sparkline.md`
- `references/cards/motion-blur-slam-in.md`
- `references/cards/news-card-desk.md`
- `references/cards/number-counter.md`
- `references/cards/number-slab-pop.md`
- `references/cards/numbered-step-stack.md`
- `references/cards/orbit-drift.md`
- `references/cards/outline-box-title.md`
- `references/cards/overexpose-flip-transition.md`
- `references/cards/parallel-items-with-host.md`
- `references/cards/particle-weld-transition.md`
- `references/cards/pencil-sketch-draw.md`
- `references/cards/per-character-rise.md`
- `references/cards/pip-zoom-box.md`
- `references/cards/pullback-cool-transition.md`
- `references/cards/push-through-transition.md`
- `references/cards/quote-bracket-pull.md`
- `references/cards/quote-card.md`
- `references/cards/quote-hold-arrow.md`
- `references/cards/rack-focus-pair.md`
- `references/cards/reticle-lock-on.md`
- `references/cards/scanline-annotate.md`
- `references/cards/scribble-annotation.md`
- `references/cards/shape-wipe-transition.md`
- `references/cards/slab-punch-title.md`
- `references/cards/slow-pull-reveal.md`
- `references/cards/slow-push-in.md`
- `references/cards/soft-blur-in.md`
- `references/cards/source-converge.md`
- `references/cards/speed-slab-title.md`
- `references/cards/split-60-40-story.md`
- `references/cards/split-compare-slider.md`
- `references/cards/split-text-stagger.md`
- `references/cards/stack-fan-out.md`
- `references/cards/stage-keyframe-tour.md`
- `references/cards/step-timeline-vertical.md`
- `references/cards/still-layout-relay.md`
- `references/cards/strike-and-replace.md`
- `references/cards/subscribe-cta.md`
- `references/cards/sway-parallax.md`
- `references/cards/terminal-typing-log.md`
- `references/cards/tilt-3d-page.md`
- `references/cards/timeline-photo-strip.md`
- `references/cards/title-demote-to-label.md`
- `references/cards/tracking-in.md`
- `references/cards/type-contrast-emphasis.md`
- `references/cards/typewriter-reveal.md`
- `references/cards/ui-flow-theater.md`
- `references/cards/ui-prop-theater.md`
- `references/cards/unit-grid-proportion.md`
- `references/cards/whip-pan-transition.md`
- `references/cards/word-relay-filmstrip.md`
- `references/cards/word-slot-cycle.md`
- `references/cards/x-follow-card.md`
- `references/cinematography.md`
- `references/demo-spec.md`
- `references/design-language.md`
- `references/host-footage.md`
- `references/layout.md`
- `references/review-protocol.md`
- `references/schematic.md`
- `references/semantic-annotation.md`
- `references/shot-design.md`
- `references/shotbook-example.md`
- `references/taxonomy.md`
- `runtime/smoke/Root.tsx`
- `runtime/smoke/index.ts`
- `runtime/test_runtime.py`
- `runtime/tsconfig.json`
- `scripts/beat_gap_check.py`
- `scripts/beat_lint.py`
- `scripts/card_lint.py`
- `scripts/card_match.py`
- `scripts/cards_index.py`
- `scripts/contact_sheet.py`
- `scripts/face_bbox.py`
- `scripts/fetch_icons.py`
- `scripts/frame_signature.py`
- `scripts/freeze_probe.py`
- `scripts/make_timing.py`
- `scripts/motion_check.py`
- `scripts/preflight.py`
- `scripts/qa_extract.py`
- `scripts/semantic_annotate.py`
- `scripts/sfx_check.py`
- `scripts/shotbook_parse.py`
- `scripts/test_fish_regressions.py`
- `scripts/test_tts_fishaudio.py`
- `scripts/timestamps_cpu.py`
- `scripts/tts_fishaudio.py`
- `scripts/voice_trim.py`
- `scripts/workbench_contract_lint.py`
- `template/README.md`
- `template/cards/alt-block-lines.tsx`
- `template/cards/bar-chart-growth.tsx`
- `template/cards/bed-echo-blur.tsx`
- `template/cards/behind-text-title.tsx`
- `template/cards/black-slam-transition.tsx`
- `template/cards/callout-line-label.tsx`
- `template/cards/caret-wipe-transition.tsx`
- `template/cards/chapter-progress-list.tsx`
- `template/cards/chapter-title-card.tsx`
- `template/cards/chart-grow.tsx`
- `template/cards/chat-gpt.tsx`
- `template/cards/chat-message-flow.tsx`
- `template/cards/chevron-lower-third.tsx`
- `template/cards/chip-grid-single-select.tsx`
- `template/cards/claude-code.tsx`
- `template/cards/color-slam-beat-card.tsx`
- `template/cards/converging-arrows.tsx`
- `template/cards/corner-bracket-frame.tsx`
- `template/cards/count-badge-title.tsx`
- `template/cards/countdown-arc-scatter.tsx`
- `template/cards/crash-zoom-punch.tsx`
- `template/cards/cursor-actor-demo.tsx`
