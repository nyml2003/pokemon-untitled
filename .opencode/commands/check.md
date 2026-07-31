---
description: 检查镜像与运行端状态
---
在仓库根目录运行 `ops check`（需要时加 `--json`）。如有 `MirrorMissing`，先运行 `ops init-mirror`；出现 `MirrorDirty` 或 `MirrorDiverged` 时停止并报告，不要手工清理镜像。报告结果与错误。
