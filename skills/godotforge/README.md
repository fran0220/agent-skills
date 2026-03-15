# GodotForge

AI 驱动的 Godot 4.x 游戏开发技能。通过 Skill（知识层）+ CLI（能力层）让 AI Agent 完成从概念到发布的游戏开发全流程。

## 安装

```bash
# 1. 安装 CLI
pip install godot-forge

# 2. 安装 Skill（符号链接）
ln -s /path/to/agent-skills/skills/godotforge ~/.config/amp/skills/godotforge

# 3. 确保 Godot 在 PATH 中
godot --version  # 需要 4.4+
```

## 架构

```
Skill（本目录）          CLI（cli/godot-forge/）
知识层                   能力层
教 Agent 做什么           让 Agent 能操作
────────────            ─────────────────
SKILL.md                godot-forge project init
reference/workflow.md   godot-forge scene create
knowledge/*.md          godot-forge engine run
                        godot-forge export build
```

## 工作流概览

19 步从零到发布：

1. 概念设计 / GDD
2. 项目初始化
3. 输入映射 / Autoload / Schema
4. 数字资产生成
5. 资产导入与处理 🔧
6. 场景构建
7. 脚本编写
8. 关卡设计
9. UI 构建
10. 音频集成
11. 动画制作
12. Shader / VFX
13. 物理 / 碰撞
14. 存档系统
15. 国际化
16. 测试 🔧
17. 性能分析 🔧
18. 构建导出 🔧
19. 发布

🔧 = 必须调用 Godot 引擎
