# Slide Deck Generator

AI 驱动的演示文稿生成工具。输入内容 → 自动生成专业幻灯片图片 → 合并为 PPTX / PDF。

## 快速开始

### 1. 安装依赖

```bash
pip install google-genai python-pptx Pillow requests
```

### 2. 配置环境变量

在 `.agents/skills/slide-deck/` 目录下创建 `.env` 文件（参考 `.env.example`）：

```env
# Gemini API（必需）
GOOGLE_API_KEY=your-google-api-key

# 可选 — Gemini API 代理地址（默认直连 Google 官方）
# GOOGLE_API_BASE=https://aihubmix.com/gemini

# 百度 OCR（可选 — 仅 editable PPTX 需要）
BAIDU_OCR_API_KEY=your-baidu-api-key
BAIDU_OCR_SECRET_KEY=your-baidu-secret-key
```

**`.env` 查找优先级**：项目根目录（向上查找）→ skill 目录（兜底）。

### 3. 通过 Agent 使用

```
/slide-deck path/to/content.md
/slide-deck path/to/content.md --style sketch-notes
/slide-deck path/to/content.md --lang zh --slides 15
/slide-deck path/to/content.md --editable
```

## 脚本说明

所有脚本位于 `scripts/` 目录，Python 3.9+，可独立运行。

| 脚本 | 用途 | 环境变量 |
|------|------|----------|
| `generate-slide-images.py` | 从 outline 生成幻灯片图片 | `GOOGLE_API_KEY` |
| `merge-to-pptx.py` | 合并图片为 PPTX（纯图片 + speaker notes） | 无 |
| `merge-to-pdf.py` | 合并图片为 PDF | 无 |
| `make-editable-pptx.py` | 图片转可编辑 PPTX（OCR + 去字 + 样式重建） | 全部四个 |
| `fix-existing-pptx.py` | 修复已有 PPTX 的文字重叠/背景残影 | 无 |

### 独立使用示例

```bash
SCRIPTS=.agents/skills/slide-deck/scripts

# 从 outline 生成指定幻灯片
python3 $SCRIPTS/generate-slide-images.py outline.md --slides 1,3,5
python3 $SCRIPTS/generate-slide-images.py outline.md --all --parallel 3

# 合并图片为 PPTX / PDF
python3 $SCRIPTS/merge-to-pptx.py slide-deck/my-topic/
python3 $SCRIPTS/merge-to-pdf.py slide-deck/my-topic/

# 图片转可编辑 PPTX（单张 / 整套）
python3 $SCRIPTS/make-editable-pptx.py slide image.png
python3 $SCRIPTS/make-editable-pptx.py deck slide-deck/my-topic/
python3 $SCRIPTS/make-editable-pptx.py deck slide-deck/my-topic/ --slides 2,5 --skip-inpaint

# 修复已有 PPTX
python3 $SCRIPTS/fix-existing-pptx.py input.pptx -o fixed.pptx

# 清理中间缓存
python3 $SCRIPTS/make-editable-pptx.py clean slide-deck/my-topic/
```

## 输出目录结构

```
slide-deck/{topic-slug}/
├── source-{slug}.md          # 源内容
├── outline.md                # 大纲（含样式指令）
├── prompts/                  # 每页 prompt
│   ├── 01-slide-cover.md
│   └── 02-slide-{slug}.md
├── 01-slide-cover.png        # 生成的幻灯片图片
├── 02-slide-{slug}.png
├── {topic-slug}.pptx         # 纯图片 PPTX
├── {topic-slug}-editable.pptx # 可编辑 PPTX（--editable）
└── {topic-slug}.pdf
```

## 样式预设

| 预设 | 适用场景 |
|------|----------|
| `blueprint` (默认) | 架构、系统设计 |
| `corporate` | 投资人 deck、商务提案 |
| `sketch-notes` | 教育、教程 |
| `dark-atmospheric` | 娱乐、游戏 |
| `minimal` | 高管简报 |
| `bold-editorial` | 产品发布、keynote |
| `notion` | 产品演示、SaaS |
| `pixel-art` | 游戏、开发者演讲 |
| `scientific` | 生物、化学、医学 |
| `watercolor` | 生活方式、艺术 |

完整列表和自定义维度见 `SKILL.md`。

## 用户偏好

可在以下位置创建 `EXTEND.md` 保存默认配置：

- `.slide-deck/EXTEND.md` — 项目级
- `$HOME/.slide-deck/EXTEND.md` — 用户级

## Editable PPTX 流水线

`make-editable-pptx.py` 将幻灯片图片转为可编辑文本框：

```
图片 → 百度 OCR 定位文字 → 裁切元素 → Gemini 去字（inpaint）
     → 双路样式提取（颜色/粗体/对齐） → PPTX 重建（clean_bg + 透明文本框）
```

中间文件（`*-ocr.json`, `*-styles.json`, `*-clean-bg.png`, `*-crops/`）自动缓存，重复运行只处理未缓存的幻灯片。`--force` 强制重新处理，`clean` 子命令清理全部缓存。
