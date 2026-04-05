---
name: asset-gateway
description: "Unified asset generation and post-processing CLI for images, video, audio, music, TTS, voice cloning, 3D, text, and image tools. Use when an agent needs to create or transform assets through one stable command surface."
---

# Asset Gateway

`asset-gateway` is the single CLI for asset generation and post-processing. Agents should think in terms of asset category and command, not provider choice.

## Install & Auth

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>
asset-gateway auth status
```

Default gateway: `https://upload.xiaomao.chat`. Override with `--gateway-url` or `ASSET_GATEWAY_URL`.

## 能力总览

| 品类 | 适用场景 | 主要命令 |
|------|----------|----------|
| 图片 | 文生图、改图、参考图、局部修改、扩图 | `generate image` |
| 视频 | 文生视频、图生视频 | `generate video` |
| 音效 | SFX、BGM | `generate audio` |
| 音乐 | 纯音乐片段生成 | `generate music` |
| 语音 | TTS、多语言、指令式说话风格 | `generate tts` |
| 声音身份 | 语音克隆、语音设计 | `voice clone`, `voice design` |
| 3D | 文生/图生 3D，后续 rig/animate/convert | `generate model`, `process3d ...` |
| 精灵动画 | 当前分支仍是逐帧生成 + 手动拼合；批量/compose/3D 渲染能力尚未合入 | 见“精灵动画工作流” |
| 文本 | 单次文本生成 | `generate text` |
| 图片工具 | 裁剪、缩放 | `process ...` |

### Decision Guide

| 用户想要什么 | 用什么命令 | 常用参数 |
|--------------|------------|----------|
| 生成图片 | `generate image` | `--prompt`, `--size`, `--output-dir` |
| 编辑已有图片 | `generate image` | `--prompt`, `--input`, `--output-dir` |
| 用多张参考图做合成或控角色一致性 | `generate image` | `--prompt`, `--ref`, `--output-dir` |
| 做语义蒙版修改 | `generate image` | `--prompt`, `--input`, `--edit-mode inpaint`, `--output-dir` |
| 做风格迁移 | `generate image` | `--prompt`, `--input`, `--ref`, `--edit-mode restyle`, `--output-dir` |
| 继续上一次图片编辑 | `generate image` | `--prompt`, `--session`, `--output-dir` |
| 生成视频 | `generate video` | `--prompt`, `--output-dir` |
| 用图做视频 | `generate video` | `--prompt`, `--input`, `--output-dir` |
| 生成音效或 BGM | `generate audio` | `--prompt`, `--type`, `--duration`, `--output-dir` |
| 生成音乐 | `generate music` | `--prompt`, `--duration`, `--output-dir` |
| 文本转语音 | `generate tts` | `--prompt`, `--voice`, `--language`, `--output-dir` |
| 指令式 TTS | `generate tts` | `--prompt`, `--instructions`, `--output-dir` |
| 克隆声音 | `voice clone` | `--audio`, `--name` |
| 设计新声音 | `voice design` | `--prompt`, `--preview-text`, `--name` |
| 生成 3D 模型 | `generate model` | `--prompt` 或 `--image`, `--output-dir` |
| 给 3D 绑骨或加动画 | `process3d rig`, `process3d animate` | `--task-id`, `--output-dir` |
| 转换 3D 格式 | `process3d convert` | `--task-id`, `--format`, `--output-dir` |
| 批量生成同一角色不同姿势 | 当前分支未内置 `generate batch`；先多次执行 `generate image --ref ...` | 见“精灵动画工作流” |
| 多图拼合为 sprite sheet | 当前分支未内置 `process compose`；先用 `convert` / `montage` | 见“精灵动画工作流” |
| 3D 模型渲染为精灵帧 | 当前分支未内置 `process3d render-sprites`；先用外部 DCC / Blender | 见“精灵动画工作流” |
| 裁剪/缩放透明边框、精确缩放 | `process ...` | `--input`, `--output-dir` |
| 生成精灵动画帧 | `generate image` + `process` | 见"精灵动画工作流"章节 |
| 单次文本生成 | `generate text` | `--prompt`, `--model`, `--output-dir` |

所有 `generate`、`process`、`process3d` 示例都应带 `--output-dir`，这样产物会被保存到本地。

如果命令参数要求 URL，而你的输入是本地文件，先上传：

```bash
asset-gateway upload file ./reference.png
```

## 图片

一条命令同时覆盖生成、编辑、参考图、局部修改、风格迁移和扩图。

### 生成

```bash
asset-gateway generate image --prompt "isometric village, soft morning light" --size 1792x1024 --output-dir ./assets
```

### 透明背景图

加 `--transparent` 生成带真实 Alpha 通道的 PNG（自动路由到 GPT Image，输出 RGBA 4 通道）。适合游戏精灵图、UI 图标、角色立绘等需要去背景的场景。

```bash
asset-gateway generate image --prompt "pixel art sword icon" --transparent --output-dir ./assets
asset-gateway generate image --prompt "character sprite sheet, chibi style" --transparent --size 1024x1792 --output-dir ./assets
```

> 不加 `--transparent` 时走 Gemini（快、便宜），加了走 GPT Image（原生透明，稍慢）。

### 编辑

支持 `--input`、最多 14 张 `--ref`、以及实际生效的 `--edit-mode`。

```bash
asset-gateway generate image --prompt "replace the sky with sunset clouds" --input https://example.com/scene.png --output-dir ./assets
asset-gateway generate image --prompt "put these two characters in the same cafe" --ref https://example.com/a.png https://example.com/b.png --output-dir ./assets
```

### 多轮编辑

第一次生成或编辑后，响应里会返回 `session_id`；后续继续传 `--session`。

```bash
asset-gateway generate image --prompt "create a product poster" --size 1792x1024 --output-dir ./assets
asset-gateway generate image --prompt "make the headline larger and change all text to Spanish" --session ses_abc123 --output-dir ./assets
```

### 尺寸控制

用 `--size WxH` 控制目标画幅，例如 `1024x1024`、`1792x1024`、`1024x1792`、`4096x2304`。

```bash
asset-gateway generate image --prompt "mobile splash screen" --size 1024x1792 --output-dir ./assets
asset-gateway generate image --prompt "wide landing page hero" --size 1792x1024 --output-dir ./assets
```

## 视频

用同一条命令做文生视频或图生视频。

### 文生视频

```bash
asset-gateway generate video --prompt "camera slowly panning over a misty mountain lake" --output-dir ./assets
```

### 图生视频

```bash
asset-gateway generate video --prompt "subtle cinematic camera push-in" --input https://example.com/keyframe.png --output-dir ./assets
```

## 音效

用于短音效和背景氛围音，不是纯音乐创作。

### BGM/SFX 生成

```bash
asset-gateway generate audio --prompt "sword slash impact" --type sfx --output-dir ./assets
asset-gateway generate audio --prompt "ambient medieval tavern" --type bgm --duration 30 --output-dir ./assets
```

## 音乐

用于生成完整一些的音乐片段。

### 音乐生成

```bash
asset-gateway generate music --prompt "uplifting indie game theme, warm synths" --duration 30 --output-dir ./assets
asset-gateway generate music --prompt "lofi study beat with soft piano" --duration 45 --output-dir ./assets
```

## 语音

覆盖普通 TTS、指令式 TTS、语音克隆和语音设计。

### TTS 合成

```bash
asset-gateway generate tts --prompt "欢迎来到今天的产品演示。" --voice Cherry --language Chinese --output-dir ./assets
asset-gateway generate tts --prompt "Welcome to the launch event." --voice Ethan --language English --output-dir ./assets
```

### 指令式 TTS

```bash
asset-gateway generate tts --prompt "This is the emergency broadcast." --instructions "calm, authoritative, slow pacing" --output-dir ./assets
```

### 语音克隆

```bash
asset-gateway voice clone --audio ./voice-sample.wav --name narrator_v1
asset-gateway voice list
```

### 语音设计

```bash
asset-gateway voice design --prompt "young female narrator, bright and friendly" --preview-text "Hello, welcome to our studio." --name host_v1
asset-gateway voice delete host_v1
```

## 3D 模型

先生成，再基于返回的 `tripo_task_id` 继续走后处理流水线。

### 生成

```bash
asset-gateway generate model --prompt "stylized low-poly warrior, T-pose" --face-limit 5000 --pbr --output-dir ./assets
asset-gateway generate model --image https://upload.xiaomao.chat/uploads/concept.png --face-limit 8000 --pbr --output-dir ./assets
```

也支持四视图输入：

```bash
asset-gateway generate model --multiview front.png,left.png,back.png,right.png --face-limit 5000 --pbr --output-dir ./assets
```

### 后处理流水线

先从 `generate model` 的响应中取 `metadata.tripo_task_id`，后续每一步都用新的 `tripo_task_id` 继续串联。

```bash
asset-gateway process3d rig --task-id abc-123 --spec mixamo --output-dir ./assets
asset-gateway process3d animate --task-id rig-456 --animation preset:walk --output-dir ./assets
```

常用后处理命令：

- `process3d texture`：重新贴图、换材质
- `process3d convert`：导出为 `FBX`、`USDZ`、`OBJ`、`STL`、`GLTF`、`3MF`
- `process3d reduce`：减面
- `process3d stylize`：体素、乐高等风格化
- `process3d segment`：拆分部件
- `process3d prerigcheck`：检查是否适合绑骨
- `process3d refine`：提升草模质量
- `process3d import`：导入外部模型接入这条流水线

```bash
asset-gateway process3d texture --task-id abc-123 --prompt "bright hand-painted fantasy materials" --pbr --output-dir ./assets
asset-gateway process3d convert --task-id abc-123 --format FBX --output-dir ./assets
```

并行线程里已经规划了 `process3d render-sprites`（Blender 无头渲染 GLB 动画到 PNG 帧），但当前分支尚未提供这个子命令。若要把 3D 动画转成 sprite frames，仍需在网关外部自行完成渲染。

## 文本

用于单次文本生成，不适合长对话状态管理。

### LLM 文本生成

```bash
asset-gateway generate text --prompt "Write a backstory for a desert kingdom." --output-dir ./assets
asset-gateway generate text --prompt "Describe a crafting system for a survival game." --model gpt-5.4 --output-dir ./assets
```

## 精灵动画工作流

当前分支仍使用手动工作流：先生成透明角色参考帧，再逐帧变换姿势，最后统一裁剪、缩放并用 ImageMagick 拼合。并行线程中设计过 `generate batch`、`process compose`、`process3d render-sprites`，但我核对当前代码后，这三个命令都还没有合入本分支，暂时不要在这里调用。

### 方式一：逐帧生成（当前可用，推荐）

**第 1 步：生成角色参考帧**

```bash
asset-gateway generate image --transparent --prompt "pixel art knight character, idle pose, facing right, clean edges, white outline" --size 1024x1024 --output-dir ./sprites
```

**第 2 步：上传参考帧以获取 URL**

```bash
asset-gateway upload file ./sprites/image_*.png
# 从输出中拿到 URL，如 https://upload.xiaomao.chat/uploads/xxx.png
```

**第 3 步：逐帧生成动作姿势**

每次都传 `--ref` 保持角色外观一致，只用 prompt 描述姿势变化：

```bash
asset-gateway generate image --transparent --prompt "same knight character, walk cycle frame 1, left foot forward, right arm back" --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites
asset-gateway generate image --transparent --prompt "same knight character, walk cycle frame 2, feet together, passing position" --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites
asset-gateway generate image --transparent --prompt "same knight character, walk cycle frame 3, right foot forward, left arm back" --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites
asset-gateway generate image --transparent --prompt "same knight character, walk cycle frame 4, feet together, returning" --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites
```

**第 4 步：裁剪和归一化所有帧**

```bash
# 裁剪透明边框并扩展到 2 的幂次
asset-gateway process crop --input ./sprites/image_frame1.png --mode power_of2 --output-dir ./sprites/final
asset-gateway process crop --input ./sprites/image_frame2.png --mode power_of2 --output-dir ./sprites/final
# ... 对每帧重复

# 统一缩放到目标尺寸
asset-gateway process resize --input ./sprites/final/frame1.png --width 64 --height 64 --output-dir ./sprites/64x64
```

**第 5 步（可选）：拼合 Sprite Sheet**

asset-gateway 负责逐帧生成和处理，拼合用 ImageMagick 完成：

```bash
# 水平拼合为 sprite sheet
magick ./sprites/64x64/*.png +append spritesheet.png
# 或 convert（ImageMagick 6）
convert ./sprites/64x64/*.png +append spritesheet.png
```

### 方式二：直出 Sprite Sheet（快速但精度较低）

一次调用直接生成整张 sprite sheet，适合简单像素风或对帧精度要求不高的场景：

```bash
asset-gateway generate image --transparent --prompt "sprite sheet, 4 frames horizontal strip, pixel art knight walk cycle, each frame 64x64, evenly spaced, facing right" --size 1024x256 --output-dir ./sprites
```

> 注意：AI 对网格布局的精确控制不稳定，可能需要多次重试或手工微调切割位置。

### Prompt 技巧

- 在参考帧 prompt 中明确**风格**（pixel art / hand-drawn / chibi）和**朝向**（facing right）
- 逐帧 prompt 以 "same character" 开头，只描述姿势变化
- 动作拆解越细致，帧间过渡越自然（走路 4-8 帧，攻击 3-6 帧）
- 加 "clean edges, no shadow on ground" 让后续裁剪更干净

## 图片后处理

用于已有图片的尺寸调整。当前分支内置的是裁剪和缩放；`process compose` 还没有合入，所以 sprite sheet 拼合仍需外部 ImageMagick `convert` / `montage`。

### 裁剪与缩放

```bash
asset-gateway process crop --input ./sprite.png --mode power_of2 --output-dir ./sprites
asset-gateway process resize --input ./poster.png --width 1024 --height 1024 --output-dir ./assets
```

### Sprite Sheet 拼合（当前分支的做法）

```bash
convert ./sprites/64x64/*.png +append ./sprites/spritesheet.png
montage ./sprites/dir00_frame*.png -tile 8x -geometry 64x64+0+0 -background none ./sprites/spritesheet_grid.png
```

`process compose` 已在并行线程中设计过多输入、横向/纵向/grid 三种布局和可选 frame 归一化，但我核对当前代码时，该子命令尚未出现在 Rust CLI、npm CLI、`/api/process` 或 `pipeline.rs` 里。

## 故障排查

- 先检查认证：`asset-gateway auth status`
- 先看网关和能力状态：`asset-gateway provider list`、`asset-gateway provider health`
- 长任务用 `asset-gateway job list`、`asset-gateway job status <job-id>` 跟踪
- 如果输入参数需要 URL，先用 `asset-gateway upload file` 上传本地文件
- 遇到 `UNAUTHORIZED`：重新执行 `asset-gateway auth set <token>`
- 遇到 `PROVIDER_ERROR`：先看 `provider health`，再重试，不要直接绕过网关调用底层 API
- 图片编辑失败时，优先检查 `--input`、`--ref`、`--edit-mode`、`--session` 是否匹配当前任务
- 不要省略 `--output-dir`，否则很难稳定地拿到本地落盘结果
- 不要用其他工具伪造或修补失败产物；失败就重试生成或如实报告

需要机器可读 schema 时使用：

```bash
asset-gateway describe
asset-gateway describe generate.image
asset-gateway describe process
asset-gateway describe process3d
```
