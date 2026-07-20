# 编辑器应用协议与结构化调用

> 分类：方案；最后核对：2026-07-20。

## 结论

地图、训练师和宝可梦编辑器共享一个纯 Rust 编辑器应用协议。协议定义编辑资源、结构化调用、观察结果、校验诊断和保存请求。它不定义地图格子、训练师队伍或宝可梦字段。

每种编辑器保留自己的纯 core 和类型化命令。GUI、CLI 和模型调用方都通过同一个协议发送请求。Ramus 负责注册、授权、校验并路由协议请求。文件、目录、图像、窗口和 GPU 只存在于 runtime adapter。

## 目标

1. 让 GUI、CLI、远程模型和本地模型调用同一组编辑意图。
2. 让调用方使用资源逻辑 ID，不接触 workspace 路径或平台路径。
3. 让每个编辑器保留强类型业务命令和独立不变量。
4. 让保存、资源读取和资产加载有同一套受控能力边界。
5. 用地图、训练师和宝可梦三个编辑器验证协议，不把试点限定为单一内容类型。

## 非目标

- 不把不同编辑器的领域字段塞进一个总状态结构。
- 不把文件系统、Winit、WGPU 或资源路径放进 core 或协议 crate。
- 不让模型直接获得任意文件路径、进程、网络或原始 shell 能力。
- 不把 Ramus Provider 变成持久状态所有者。

## 分层

```text
GUI input / CLI JSON Lines / local model / remote model
                       |
                       v
                editor-ramus-adapter
  capability, schema validation, structured routing
                       |
                       v
              editor-application protocol
  EditorCall, EditorDocumentId, EditorResponse, diagnostics
                       |
       +---------------+----------------+
       |               |                |
       v               v                v
map-editor-core  trainer-editor-core  pokemon-editor-core
 typed map API     typed trainer API   typed Pokemon API
       |               |                |
       +---------------+----------------+
                       |
                       v
          editor resource adapters and runtimes
  document ID -> approved path, load/save, image/UI assets, window
```

## Rust 结构化调用协议

`editor-application` 定义以下稳定结构：

- `EditorKind`：`Map`、`Trainer`、`Pokemon`。
- `EditorDocumentId`：调用方可见的逻辑资源身份。它不能是文件路径。
- `EditorCall`：协议版本、资源身份、操作和类型化 payload 的外层信封。
- `EditorOperation`：`Inspect`、`Validate`、`Command`、`Save`。
- `EditorResponse`：快照、诊断、已应用命令或保存请求。
- `EditorDiagnostic`：稳定 code、目标和用户可读消息。

具体 core 的命令仍是 Rust enum，并经 `serde` 编码进入 `EditorCall`。例如地图使用现有 `EditorVirtualCommand`，训练师使用 `TrainerEditCommand`，宝可梦使用 `PokemonEditCommand`。协议只负责编排和传输，不把它们降级为字符串命令。

协议调用是无副作用的，直到 runtime 收到 `Save`。`Save` 只能请求保存；资源 adapter 在目标文件写入前再次校验并返回结果。失败不会改变内存文档或磁盘文件。

## Ramus 与统一资源协议

`editor-ramus-adapter` 为每个编辑器资源注册相同的四类方法：

| Ramus 方法 | 协议操作 | 所需能力 | 说明 |
| --- | --- | --- | --- |
| `/editor/resource open` | 装载请求 | `Read` | 让 runtime 以逻辑 ID 装载文档。 |
| `/editor/resource inspect` | `Inspect` | `Read` | 返回可供 GUI、CLI 或模型读取的结构化状态。 |
| `/editor/resource validate` | `Validate` | `Read` | 返回领域校验诊断，不写文件。 |
| `/editor/command execute` | `Command` | `Write` | 接受 schema 校验后的结构化命令。 |
| `/editor/resource save` | `Save` | `Write` | 只发出保存意图，由 runtime 的资源 adapter 执行。 |

Ramus catalog 也提供 `Discover` 和 `Complete`。人类 GUI、CLI、远程模型和本地模型使用相同 principal/capability 模型；运行时可为不同用户或模型发放更窄的资源前缀与操作能力。

资源协议不把路径作为 Ramus 参数。运行时先把受信任的 `EditorDocumentId` 解析为白名单内容文件，再调用加载或保存端口。这样模型不能通过编辑协议读取或覆盖任意文件。

## 资源管理

`editor-resource-adapter` 负责三个动作：

1. 将逻辑 document ID 解析为该编辑器允许的内容位置。
2. 读取文本或二进制资源并交给对应 core 解析。
3. 在 core 已校验且收到保存意图后原子写回该资源。

窗口 runtime 另外从同一 adapter 取得 UI 所需的白色基础资源和该编辑器自己的图像资源。协议中的资源身份与渲染资源身份分开，避免把 atlas、GPU ID 或文件路径泄漏给调用方。

## 试点

| 编辑器 | 既有核心 | 试点改动 | GUI | CLI |
| --- | --- | --- | --- | --- |
| 地图 | `map-editor-core` 的 `EditorVirtualCommand` | 包装为 `EditorCall`，通过统一 router 调用。 | 现有 `map-editor` 继续调用同一 core。 | `map-editor-cli` 继续 JSON Lines，并接受协议信封。 |
| 训练师 | 新建 `trainer-editor-core` | 姓名、队伍、脚本的强类型编辑命令与校验。 | 新 `trainer-editor` Windows GUI。 | 新 `trainer-editor-cli` JSON Lines。 |
| 宝可梦 | 新建 `pokemon-editor-core` | 作者定义的宝可梦目录、字段编辑和校验。 | 新 `pokemon-editor` Windows GUI。 | 新 `pokemon-editor-cli` JSON Lines。 |

地图已有 GUI/CLI 入口，因此试点不重写其工作台。训练师与宝可梦的 GUI 使用同一窗口、UI frame、原生资源和事件循环约定；它们不会复制业务转换。

## 实施顺序

1. 新建 `editor-application`，先测试协议值对象、调用信封和诊断。
2. 新建训练师与宝可梦 core，将可编辑内容和命令从 runtime 中移出。
3. 新建 `editor-ramus-adapter`，把三类结构化命令注册为相同资源操作。
4. 新建 `editor-resource-adapter`，只允许已注册 document ID 的读取和保存。
5. 迁移地图 CLI，新增训练师、宝可梦 CLI。
6. 新增训练师、宝可梦 GUI，并将它们的按钮、文本输入转换为同一协议调用。
7. 扩展 `ops` 的 Windows target，验证三个 GUI；更新 current 文档。

## 完成条件

- 三类编辑器都可构造、执行和序列化 `EditorCall`。
- 对同一内容，GUI、CLI 和 Ramus 调用得到相同 core 结果。
- Ramus 拒绝未授权资源、未注册方法和不符合 schema 的参数。
- 保存只能通过已注册 document ID 解析的资源 adapter 执行。
- 地图、训练师、宝可梦 core 的非法命令均不改变原状态。
- GUI/CLI 不直接持有领域转换；其职责仅为输入输出和 runtime 生命周期。
