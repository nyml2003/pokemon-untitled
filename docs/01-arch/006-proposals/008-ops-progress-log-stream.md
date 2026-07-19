# Ops 进度日志流方案

> 状态：已实施，待 Windows 原生验收

## 结论

`ops` 的长任务需要持续输出结构化进度事件。人类可读日志写入标准错误；`--json` 的最终结果继续单独写入标准输出。

这样终端、编辑器和自动化调用方都能在任务运行期间看到当前阶段、子进程输出和失败原因，不必等到命令结束。

## 问题

`ops init-mirror`、`ops sync`、`ops build game-host` 和 `ops run game-host` 可能执行数分钟：Git 首次克隆、Git LFS 下载、Windows 构建和游戏运行都属于长阶段。

目前命令只在开始或结束时返回有限信息。底层 Git、Git LFS 和 Windows 运行端的输出不能稳定地逐行转发。调用方无法区分“仍在下载”“正在构建”“窗口仍在运行”和“命令卡住”。

## 目标

- 每个长任务开始后立即声明阶段。
- 子进程的标准输出和标准错误按到达顺序持续转发。
- `--json` 保持标准输出只包含一个最终 JSON 结果。
- 进度事件可以由终端实时显示，也可以被 IDE、CI 或任务面板解析。
- 失败事件必须给出阶段、错误代码和可执行的下一步。
- 日志不能泄露令牌、密码、完整认证 URL 或环境变量值。

## 非目标

- 不将任意 shell 命令暴露给 CLI。
- 不把完整构建日志写入 Git 仓库。
- 不在 `ops` 内实现持久日志服务或远程日志上传。
- 不改变 Git 镜像只接受已推送远端提交的边界。

## 输出契约

所有命令共用 `ProgressReporter` 端口。应用层报告阶段和语义事件；Git 适配器、Windows 运行端适配器报告其受控子进程输出。

人类可读模式把每条事件写到标准错误：

```text
[12:04:18] sync.fetch  获取 origin/master
[12:04:22] sync.fetch  remote: Enumerating objects: 42, done.
[12:04:23] sync.lfs    下载 LFS 对象 18/240
[12:04:39] sync.done   镜像已更新：a52a0b2 -> 5e43d88
```

`--json` 模式下，事件使用一行一个 JSON 对象（JSON Lines）写到标准错误；标准输出只在命令结束时写一个最终结果对象：

```json
{"type":"progress","timestamp":"2026-07-20T12:04:18Z","stage":"sync.fetch","message":"获取 origin/master"}
{"type":"output","timestamp":"2026-07-20T12:04:22Z","stage":"sync.fetch","stream":"stderr","message":"remote: Enumerating objects: 42, done."}
{"type":"result","status":"ok","mirror_before":"a52a0b2","mirror_after":"5e43d88"}
```

调用方应以事件的 `timestamp` 和接收顺序显示日志，不依赖不同子进程的时钟排序。

## 事件模型

每个进度事件至少包含以下字段。

| 字段 | 含义 |
| --- | --- |
| `type` | `progress`、`output`、`warning` 或 `error`。 |
| `timestamp` | `ops` 写出事件时的 UTC 时间。 |
| `stage` | 固定阶段标识。 |
| `message` | 面向人的简短说明或子进程原始单行输出。 |

`output` 额外包含 `stream`，值为 `stdout` 或 `stderr`。`error` 额外包含稳定的 `code` 和可选 `remediation`。敏感参数必须在适配器中脱敏后才能填入 `message`。

阶段标识固定如下：

| 命令 | 阶段 |
| --- | --- |
| `init-mirror` | `mirror.validate`、`mirror.clone`、`mirror.lfs`、`mirror.verify` |
| `sync` | `sync.inspect`、`sync.fetch`、`sync.fast_forward`、`sync.lfs`、`sync.verify`、`sync.done` |
| `build game-host` | 同步阶段，随后 `build.start`、`build.output`、`build.done` |
| `run game-host` | 同步和构建阶段，随后 `run.start`、`run.output`、`run.exit` |

`stage` 是对外契约。新增阶段可以增加，既有阶段的含义不得改写。

## 子进程转发

适配器必须用管道分别读取子进程的标准输出和标准错误。读取到一个完整行就立即发出 `output` 事件，不能等待子进程结束后再批量输出。

部分工具会在非 TTY 管道中缓冲，或用回车更新同一行进度。适配器应保留换行分割，并把独立的回车进度转换为普通事件。没有文本输出超过 15 秒时，应用层发出心跳事件，说明仍在等待的阶段和已耗时。

子进程退出后，适配器必须先排空两个输出流，再发出 `done` 或 `error` 事件。非零退出码转换为结构化 `OpsError`，同时保留最后 40 条已脱敏事件作为诊断上下文。

## 取消和失败

调用方中断 `ops` 时，应用层向当前受控子进程发送终止信号，继续转发其剩余输出，并以 `Cancelled` 返回。不得启动额外清理命令，不得自动删除镜像目录或重置 Git 状态。

Git 克隆、LFS 下载和 fast-forward 的失败分别保留为 `GitSyncFailed`、`GitLfsUnavailable` 或 `MirrorDiverged`。原生构建和游戏进程失败保留其 Windows 退出码，并附带最后输出。

## 分层与实现

`domain` 只定义 `ProgressEvent`、阶段标识和错误上下文上限。

`application` 在命令边界创建 reporter，并在同步、构建和运行之间切换阶段。它不读取管道，也不格式化终端文本。

`ports` 定义 `ProgressReporter` 和可流式执行固定操作的受限进程接口。接口不能接受任意命令文本。

`adapters` 负责进程管道、行拆分、脱敏、心跳和终端/JSON Lines 格式化。CLI 将最终结果写到标准输出，所有过程事件写到标准错误。

## 实施顺序

1. 定义事件模型和 `ProgressReporter` 端口，为内存 reporter 添加单元测试。
2. 改造 Git 镜像适配器，流式转发 clone、fetch、LFS 和 fast-forward 输出。
3. 改造 Windows 运行端适配器，流式转发构建和游戏进程输出。
4. 增加文本 reporter 与 JSON Lines reporter，保证 `--json` 标准输出只有最终结果。
5. 增加 15 秒心跳、取消和最后 40 条诊断上下文。
6. 在首次镜像初始化、增量同步、构建失败和游戏退出四个场景验收输出。

## 已实施

- `ProgressEvent`、`ProgressReporter`、文本 reporter 与 JSON Lines reporter 已加入 `ops`。
- `init-mirror`、`sync`、Windows 私有运行端均通过管道实时转发标准输出和标准错误。
- 无输出超过 15 秒时，当前阶段会输出心跳事件。
- 远端提交未变化时，`sync` 跳过 Git LFS，避免每次同步扫描全部 LFS 指针。
- 进程失败会保留末尾 40 条脱敏输出；`Ctrl+C` 会终止当前受控进程，排空已到达的输出后返回 `Cancelled`。
- Python 单元测试已覆盖双流转发、心跳、JSON Lines、末尾输出和认证 URL 脱敏。

尚未执行 Windows 原生构建和游戏窗口验收。该验收应在一个已推送的提交上，通过 `ops run game-host` 完成。

## 验收标准

- `ops init-mirror` 在 Git LFS 下载期间至少每 15 秒输出一条阶段或心跳事件。
- `ops sync` 可以实时看到 fetch、fast-forward 和 LFS 输出；目标提交不变时也明确报告无需更新。
- `ops run game-host` 可以持续看到同步、构建和运行输出，直到游戏进程退出。
- `ops run game-host --json` 的标准输出能被单次 JSON 解析；标准错误能按行解析为事件。
- 失败结果含稳定错误代码、发生阶段和最后 40 条已脱敏输出；不会自动覆盖或删除镜像内容。
- 测试覆盖正常输出、交错的标准输出/错误、静默心跳、非零退出、取消和敏感内容脱敏。
