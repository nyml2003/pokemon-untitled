# Windows 镜像同步

> 分类：现状；最后核对：2026-07-20。
> 依据：`ops-first-workflow` 的镜像语义与恢复规则。

## 同步输入是远端提交

WSL 工作区是唯一编辑源。Windows Git 镜像只同步配置远端分支上的提交：未提交修改和未推送提交都不会进入镜像。`ops sync` 不会替开发者提交或推送 WSL 工作区。

正常顺序是：

```text
在 WSL 修改和验证
  -> 明确提交并推送
  -> ops sync
  -> Windows 镜像更新到配置远端分支
  -> 原生构建或运行
```

新镜像先用 `ops init-mirror` 创建。日常更新只用 `ops sync`；不要将镜像作为额外工作树进行手动编辑。

## 停止而不是强制修复

| 结果 | 含义 | 当前动作 |
| --- | --- | --- |
| `MirrorMissing` | 未初始化或镜像目录不可用。 | 使用 `ops init-mirror`，不手工复制目录。 |
| `MirrorDirty` | 镜像有本地未提交变化。 | 停止，人工处理镜像状态。 |
| `MirrorDiverged` | 镜像与目标分支分叉。 | 停止，人工处理分叉。 |
| `GitSyncFailed` | 拉取或同步失败。 | 保留阶段事件，修复远端、网络或提交问题后重试。 |

ops 不会自动 stash、clean、merge、rebase 或 reset 镜像。这个限制防止同步流程覆盖 Windows 侧尚未确认的内容。
