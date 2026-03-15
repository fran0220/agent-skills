# CLI Tools

可安装的命令行工具集合，被 `skills/` 中的 Skill 引用。

每个 CLI 是独立的 Python 包，通过 `pip install` 安装。

## 目录结构

```
cli/
└── <cli-name>/
    ├── pyproject.toml
    ├── README.md
    ├── <package_name>/
    │   ├── __init__.py
    │   ├── cli.py
    │   ├── commands/
    │   ├── core/
    │   └── utils/
    └── tests/
```

## 开发约定

参见根目录 [AGENTS.md](../AGENTS.md) 中的 CLI 开发约定。
