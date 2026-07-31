---
description: 同步 Windows Git 镜像
---
在仓库根目录运行 `ops sync`，更新 Windows 镜像到配置远端分支的当前提交。`ops sync` 只更新镜像提交，不会提交或推送 WSL 工作区。如有 `MirrorMissing` 先运行 `ops init-mirror`；出现 `MirrorDirty` 或 `MirrorDiverged` 停止并报告。报告结果。
