# Git 驱动的 Windows 运行镜像方案

> 状态：已实施，待 Windows 原生验收

## 结论

Windows 运行目录改为独立 Git 工作区，不再由 `ops` 逐文件复制 WSL 源树。

可运行版本必须是已经通过 WSL 单元测试并推送到远端的提交。Windows 镜像只快进到这个提交，再原生构建和运行 `game-host`。

这会消除每次运行都复制 `assets/` 的问题。首次克隆仍需要下载完整仓库；后续只传输 Git 中新增或变更的对象。

## 历史问题

Git 镜像改造前，`ops sync` 枚举源工作区中的全部受管文件，并把每个文件加入复制计划。它不比较镜像版本，也不比较文件内容。

`ops run game-host` 每次都会先执行这个同步。项目包含大量图片和数据文件，因此首次同步和后续运行都会长时间占用 WSL 到 NTFS 的文件复制链路。

逐文件复制还有一个隐患：Windows 目录不是 Git 工作区，无法明确回答原生窗口实际运行的是哪个提交。

## 目标流程

```text
WSL 工作区
  修改 -> ops format --check -> ops test -> git commit -> git push
                                                    |
                                                    v
Windows Git 镜像
  git fetch -> git pull --ff-only -> ops run game-host
```

1. 所有编辑和单元测试仍在 WSL 源工作区完成。
2. 测试通过后，开发者创建提交并推送到约定分支。
3. Windows 镜像只拉取远端已推送的提交，并且只允许 fast-forward。
4. `game-host` 从 Windows Git 镜像构建和运行。
5. `target/`、日志和崩溃产物留在 Windows，Git 不跟踪也不回写 WSL。

## 版本边界

Windows 原生验收的是已推送提交，不是 WSL 当前工作树。

- 配置远端分支的当前提交是唯一运行契约。
- WSL 工作树不要求干净。未提交修改和已提交但尚未推送的本地提交都不进入 Windows 镜像，也不阻塞运行。
- Windows 镜像存在已跟踪的本地修改时，拉取前必须失败，不能自动丢弃、暂存或合并。
- Windows 镜像只能快进到配置的远端分支，不能接受任意提交哈希、分支名或 Git 参数。

该边界适合原生验收和可复现问题报告。需要验收本地改动时，先将其推送到配置分支，再运行 Windows 原生验收。

## 镜像初始化

旧的 `C:\Users\nyml\projects\pokemon-untitled` 是 ops 复制产生的非 Git 目录。Git 不能安全地直接克隆到这个非空目录。

迁移必须创建新的空镜像目录，例如 `C:\Users\nyml\projects\pokemon-untitled-native`：

1. 修改 `ops.local.json`，让 `mirror` 指向新的空目录。
2. 从 WSL 执行显式的 `ops init-mirror`。
3. `ops init-mirror` 从配置读取固定远端和固定分支，在镜像目录克隆仓库并初始化 Git LFS 内容。
4. 命令校验仓库远端、当前分支和 `HEAD` 都符合配置。
5. 运行一次 Windows 原生构建和窗口验收。
6. 验收完成后，人工决定是否删除旧复制镜像。

旧镜像不得由迁移命令自动删除或覆盖。

## Ops 改造

`ops` 继续是 WSL 的唯一开发和原生运行入口。Git 是 `ops` 的一个受控传输适配器，不向 CLI 开放任意 Git 子命令或参数。

### 本机配置

`ops.local.json` 新增镜像版本来源：

```json
{
  "mirror": {
    "wsl_mount_root": "/mnt/c/Users/nyml/projects/pokemon-untitled-native",
    "windows_root": "C:\\Users\\nyml\\projects\\pokemon-untitled-native",
    "remote": "origin",
    "branch": "master"
  }
}
```

`remote` 和 `branch` 是受校验的配置值。它们不是 CLI 参数。

当前验收分支固定为 `master`。项目目前由单人维护，不引入额外验收分支。多人协作开始后，再评估是否拆出 `native-verify` 分支。

### 新的命令语义

| 命令 | 行为 |
| --- | --- |
| `ops init-mirror` | 只在配置的镜像目录为空时克隆固定远端和分支，并初始化 Git LFS。 |
| `ops doctor` | 校验镜像是 Git 工作区、远端 URL、上游分支、Git LFS 可用性和 Windows Python。 |
| `ops check` | 只报告 WSL `HEAD`、远端目标提交、镜像 `HEAD`、脏工作区状态和是否可 fast-forward。 |
| `ops sync` | 获取远端并把镜像 fast-forward 到配置分支。它实时输出阶段和子进程日志；只在 LFS 指针或 `.gitattributes` 变化时更新 LFS 对象。 |
| `ops build game-host` | 先执行 Git 同步，再在 Windows 镜像中构建固定目标。 |
| `ops run game-host` | 先执行 Git 同步，再在 Windows 镜像中构建和运行固定目标。 |

`ops sync` 不负责 WSL 的提交或推送。提交和推送仍由开发者在 WSL 工作区明确执行，避免 ops 自动创建提交或把未审核修改发布到远端。

镜像目录不存在时，`ops doctor`、`sync`、`build` 和 `run` 必须返回 `MirrorMissing`，并提示先运行 `ops init-mirror`。镜像目录非空但不是已验证的 Git 镜像时，命令必须返回 `UnsafeMirror`，说明 ops 不会覆盖该目录。远端、分支或 Git LFS 校验失败时，错误必须报告期望值和实际值。

### 分层与端口

`domain` 新增镜像版本状态和同步计划。它只比较提交 ID、分支关系和可执行状态，不调用 Git。

`application` 编排以下顺序：读取 WSL 和镜像版本状态、拒绝脏镜像或非 fast-forward、请求 Git 适配器更新镜像、重新读取并验证最终提交。

`ports` 新增受限的 `GitMirror` 接口。接口只包含读取状态、初始化、fetch 和固定分支 fast-forward，不接受命令字符串。

`adapters` 使用 Windows 镜像目录中的 Git 执行上述固定操作，并把退出状态转换为现有结构化错误。CLI 只显示进度、提交 ID 和诊断。

## 安全规则

- 只允许配置的远端、分支和镜像目录。
- 拉取使用 fast-forward-only；发生分叉时停止并报告，不合并、不变基、不强制重置。
- 镜像已跟踪文件有修改时停止；不自动 `stash`、`clean` 或覆盖文件。
- `target/` 和其他 Git 忽略产物可以保留，不能因此阻塞更新。
- Git LFS 对象未下载、校验失败或资源缺失时，原生构建前停止。
- Windows 侧不提供开放式 Git 或构建命令。

## 进度输出

正常输出按阶段显示：读取镜像状态、获取远端、快进提交、按需检查 LFS、校验完成、开始原生构建或开始运行。Git 与 Windows 原生构建的标准输出和错误持续转发；静默超过 15 秒时会输出心跳。

使用 `--json` 时，进度以 JSON Lines 写入标准错误；最终结果写入标准输出。最终结果至少包含：

```json
{
  "source_head": "<commit>",
  "remote_head": "<commit>",
  "mirror_before": "<commit>",
  "mirror_after": "<commit>",
  "fast_forwarded": true
}
```

## 实施状态

Git 镜像、显式初始化、受限 fast-forward、按需 LFS、结构化日志和 Python 单元测试均已实施。Windows 原生构建与游戏窗口验收尚未执行。

## 实施阶段

1. 增加 Git 镜像领域模型、端口和内存测试。覆盖相同提交、可快进、镜像脏、分叉和远端缺失。
2. 增加 Windows Git 适配器和真实临时仓库测试。验证只执行固定操作，且失败返回结构化错误。
3. 增加显式 `ops init-mirror`。覆盖目录缺失、空目录、非空非 Git 目录、错误远端、错误分支和 Git LFS 缺失。
4. 将 `ops check`、`doctor`、`sync`、`build` 和 `run` 切换到 Git 同步，移除逐文件复制计划。
5. 创建新的 Windows Git 镜像，配置远端与分支，完成首次克隆和 LFS 校验。
6. 在 WSL 运行格式检查和单元测试，创建并推送验收提交。
7. Windows 镜像拉取该提交，运行 `ops run game-host`，验收窗口、输入、资源加载、GPU 提交和退出状态。
8. 原生验收通过后，停止使用旧复制镜像；旧目录由人工清理。

## 验收标准

- 未修改代码时，连续两次 `ops sync` 不复制资源文件或扫描全部 LFS 指针，只报告镜像已处于目标提交。
- 修改一个未受 LFS 管理的源码文件并推送后，Windows 镜像只接收对应 Git 对象，跳过 LFS，随后可原生构建。
- 修改一个受 Git LFS 管理的资源并推送后，镜像取得对应 LFS 对象，资源加载通过。
- WSL 有未提交改动时，Windows 运行的提交 ID 仍等于远端已推送提交。
- 镜像有已跟踪修改或无法 fast-forward 时，`ops sync` 拒绝继续且不破坏镜像。
- 镜像目录缺失时，`ops init-mirror` 可初始化；非空非 Git 目录被拒绝且不被改写。
- `ops run game-host` 持续转发 Git、构建和运行日志，并在游戏退出后返回 Windows 退出码。

## 待定项

当前没有待定项。多人协作开始后，再重新评估验收分支策略。
