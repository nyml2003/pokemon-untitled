---
name: ops-first-workflow
description: 在 Pokemon Untitled 中执行或建议代码修改、格式化、测试、同步、构建、CI 验证或原生渲染运行时，统一通过 ops 工作流。用于避免开发者、文档、CI 和自动化直接调用 Cargo、PowerShell、cmd 或 Windows 原生命令。
---

# Ops First Workflow

将 `ops` 视为项目的唯一开发与验证入口。先在仓库根目录进入 `nix develop`。

## 命令选择

| 目的 | 命令 |
| --- | --- |
| 配置、镜像与运行端诊断 | `ops check`、`ops doctor` |
| 初始化新的 Windows Git 镜像 | `ops init-mirror` |
| WSL 格式检查 | `ops format --check` |
| WSL 全量单元测试 | `ops test` |
| WSL 局部单元测试 | `ops test --suite core` 或 `ops test --suite world` |
| 更新 Windows Git 镜像 | `ops sync` |
| Windows 原生构建 | `ops build game-host` |
| 同步、Windows 重建并运行渲染 | `ops run game-host` |

使用 `--json` 获取机器可读结果。命令可写为 `ops check --json`。

## 约束

- 不在面向开发者的命令、CI 配置、文档或建议中直接使用 `cargo`、`python -m tools.pokemon_ops`、PowerShell、`cmd.exe` 或 Windows 原生命令。
- 不在 WSL 直接启动 `game-host`。使用 `ops run game-host`；它会安全同步镜像，再调度 Windows 私有运行端。
- 不将 Windows Git 镜像视为编辑源或测试入口。所有单元测试在 WSL 通过 `ops test` 完成。
- Windows 原生验收只运行配置远端分支的当前提交。WSL 未提交或未推送的修改不会进入镜像。
- 新镜像必须通过 `ops init-mirror` 显式创建。`ops sync`、`ops build game-host` 和 `ops run game-host` 不会创建、覆盖或清理镜像目录。
- 不通过额外参数透传构建器命令、crate 名称或任意 shell 文本。
- 当 `ops.local.json` 缺失、`ops doctor` 失败或 ops 本身有缺陷时，先诊断或修复 ops；不要自动退回到直接构建器命令。只有在诊断 ops 本身时，才可在说明中提及底层命令，并明确它不是项目工作流。

## 完成检查

报告已执行的 ops 命令及其结果。涉及渲染时，说明 Windows 原生运行是否已实际验收；没有 Windows 环境时，明确该验证未运行。
