# 编译制品、观测与边界治理

> 状态：契约 crate 与最小流式编译已实现；缓存、构建事件和并行调度仍未实现

## 结论

DSL 工具链应采纳四项结构：稳定的契约层、显式制品键、可外接的构建事件和依赖方向测试。它们为闲时批量预编译、AI 内容审核和未来多工具协作提供基础，但不要求现在拆成大量 crate，也不要求引入通用调度器。

当前已创建 `narrative-token`、`narrative-cps` 和 `narrative-compiler` 三个纯 crate。它们只定义词法输入、CPS 程序和值类型以及最小编译器；尚未拆出 AST 或接口 catalog crate，也没有 adapter、缓存和线程池。

本设计借鉴了 Kaubo 工具链的 CPS、结构化事件和制品编排思路；只采纳适合受限游戏 DSL 的部分。[CPS IR](https://github.com/nyml2003/kaubo-features/blob/289b569cd6feca3c0ebf4b74c9c227cca644388a/docs/architecture/03-cps-ir.md)、[事件与日志](https://github.com/nyml2003/kaubo-features/blob/289b569cd6feca3c0ebf4b74c9c227cca644388a/docs/architecture/06-events-and-logging.md)和[编排架构](https://github.com/nyml2003/kaubo-features/blob/289b569cd6feca3c0ebf4b74c9c227cca644388a/docs/architecture/13-orchestration-architecture.md)是设计参考，不是代码来源。

## 一、契约层与依赖方向

以下是逻辑模块，不是立即创建的 Rust crate。它们在拥有独立不变量，且被至少两个上层消费者使用前，仍可作为现有领域模块中的独立子模块维护。

| 逻辑模块 | 稳定职责 | 不负责 |
| --- | --- | --- |
| `narrative-token` | ASCII token、源位置和词法错误 | 文件读取、资源查询 |
| `narrative-ast` | parser 的短生命周期语法结构 | 运行时解释、世界状态 |
| `narrative-cps` | `ContinuationId`、CPS 节点、控制流校验 | provider、账本、UI |
| `narrative-interface` | 脚本公开入口、参数和 effect 摘要 | 完整实现、执行状态 |

依赖方向必须保持单向：

```text
source/cache/CLI/GUI adapter
            |
            v
 parser -> validator -> CPS builder -> sealed ScriptProgram
            |              |
            v              v
     token / AST       CPS / interface contracts
            ^              ^
            |              |
      runtime, simulation, audit readers
```

`world-application`、Ramus adapter 和模型 adapter 可以消费已密封的 `ScriptProgram`，但契约层不得反向依赖它们。构建事件也不得让 parser 或 CPS crate 依赖控制台、文件系统或 GUI。

当这些逻辑模块拆为 crate 后，应增加 architecture test：契约 crate 不依赖编译阶段、运行时、adapter 或 presentation crate；各编译阶段之间不允许形成循环依赖。该测试检查 `Cargo.toml` 的依赖图，不依赖人工约定。

## 二、制品和缓存

每个可缓存产物必须有明确种类、输入摘要和版本。第一版只需要四种制品：

```rust
pub enum ScriptArtifactKind {
    Source,
    InterfaceCatalog,
    Program,
    SimulationReport,
}

pub struct ScriptArtifactKey {
    pub bundle: ContentBundleId,
    pub script: ScriptId,
    pub kind: ScriptArtifactKind,
    pub source_hash: ScriptHash,
    pub language: ScriptLanguageVersion,
    pub resource_catalog: CatalogDigest,
    pub interface_catalog: CatalogDigest,
    pub capability_catalog: CatalogGeneration,
}
```

`Program` 的 key 必须包含所有编译期可观察输入。只用脚本文件 hash 缓存会遗漏资源 ID、其他脚本接口或 capability schema 的变化，最终导致旧程序在新世界规则下继续运行。

发布规则如下：

1. 流式前端先产生接口摘要和候选 `ProgramFragment`。
2. 外层收集同一 bundle 的接口摘要，形成确定排序的 `InterfaceCatalog`。
3. linker 只在所有依赖接口可用且没有 error 时密封 `ScriptProgram` manifest。
4. 缓存只发布成功 manifest 指向的完整产物。失败的 fragment 和诊断可保留为审核资料，但绝不可被运行时加载。

脚本之间只允许通过声明的入口和接口摘要引用。循环调用、未声明入口和 effect 摘要不匹配必须在链接阶段失败。这样独立脚本可以在不同 worker 中预编译，最终只做确定性的接口链接。

## 三、纯流式前端

本项目的源前端仍以 `ByteStream -> token -> ParseEvent -> CPS fragment` 为目标。接口摘要也是事件流的一部分；它不要求把全部脚本源、完整 token 列表或 AST 常驻内存。

构建层可以为诊断和编辑体验选择保留 AST 快照，但那是 adapter 的可选索引，不是编译正确性的前置条件。不得因为引入缓存或 GUI 就把核心退化为“先读完整文件、再构造完整 AST”的唯一实现。

外部参考项目当前的 parser 使用 `&str -> Vec<Token> -> AST` 的递归下降路径。[Parser 设计](https://github.com/nyml2003/kaubo-features/blob/289b569cd6feca3c0ebf4b74c9c227cca644388a/docs/architecture/01-parser.md)因此只能作为诊断分层和 parser 组织方式的参考，不能替代本项目的纯流式要求。

## 四、构建事件与世界账本

构建核心产生结构化事件。外层决定是否显示、记录或聚合：

```rust
pub enum ScriptBuildEvent {
    SourceAccepted { script: ScriptId, hash: ScriptHash },
    InterfaceProduced { script: ScriptId, interface: InterfaceDigest },
    FragmentProduced { script: ScriptId, continuation_count: u32 },
    CacheHit { key: ScriptArtifactKey },
    Diagnostic { diagnostic: Diagnostic },
    Sealed { program: ScriptProgramRef },
    Rejected { script: ScriptId, error_count: u32 },
}
```

这些事件用于 CLI、GUI、CI、性能统计和 AI 草案审核。它们的存储可以被清理或重建，也可以只保留聚合结果。

`WorldLedgerEntry` 具有不同语义。它描述已提交的游戏事实，必须参与存档、回放、迁移和一致性检查。任何构建事件都不能改变世界；任何世界提交也不能因为日志 handler 不可用而失败。

| 数据 | 是否权威 | 可否丢弃 | 是否推进世界 |
| --- | --- | --- | --- |
| `ScriptBuildEvent` | 否 | 可以 | 否 |
| 编译诊断快照 | 否 | 可以重建 | 否 |
| `WorldLedgerEntry` | 是 | 不可以 | 已在结算点提交 |
| `WorldPatch` | 是 | 不可以 | 作为已接受世界变化生效 |

## 五、CPS 的采纳范围

运行时控制流采用基本块与显式终止语义。现有 `CpsNode` 中的 `Jump`、`Compare`、`RequestChoice`、`RequestOperation`、`Wait` 和 `End` 已覆盖第一版需要的转移、分支、外部请求、暂停和完成。

`Wait`、`RequestChoice` 和 `RequestOperation` 都是游戏意义上的 suspend 点。它们必须产出可存档的 `ScriptContinuation`，并在下一次结算点恢复；不得保留进程栈、异步 future、provider handle 或模型请求作为 continuation 的一部分。

不采纳通用语言的寄存器分配、闭包捕获、递归调用、动态分派、类型推导、垃圾回收或任意外部调用。DSL 的目标是可控叙事流程，而非承载通用计算。

## 六、渐进落地

### 第一步：保持为模块，先验证合同

1. 在 DSL crate 或 `world-event-domain` 内定义 token、CPS、接口摘要和 `ScriptArtifactKey`。
2. 为相同输入得到相同 program hash、诊断顺序和接口摘要写 fixture。
3. 通过内存 `ByteStream` 运行编译；不引入文件缓存、线程池或 GUI。

完成标准：没有 filesystem、模型服务和 UI 时，编译器仍可完成解析、校验和密封。

### 第二步：加入缓存与批处理

1. 在 adapter/CLI 层实现基于 `ScriptArtifactKey` 的文件或数据库缓存。
2. 按接口依赖图分批编译相互独立的脚本。
3. 固定输入排序、worker 结果合并和诊断排序，确保并行不改变产物 hash。

完成标准：缓存命中与未命中得到相同 `ScriptProgram`；并行和单线程得到相同 manifest 和诊断。

### 第三步：按需要拆分 crate 与审计视图

1. 当运行时、CLI 和 GUI 都消费 CPS 或接口时，再拆出契约 crate。
2. 加入 Cargo 依赖方向测试。
3. 将 `ScriptBuildEvent` 接到 CLI、GUI 和 CI；审计视图只读取同一份制品和账本。

完成标准：新增工具不会让编译核心依赖 IO；任何依赖反向变化都会被测试阻止。

## 当前不做

- 不复制外部仓库的代码；其 GitHub 仓库当前未标注许可证。
- 不引入通用任务编排平台、VFS、语言服务器或 WASM 运行时作为 DSL 第一版前置条件。
- 不让 `ScriptBuildEvent` 代替 `WorldLedgerEntry`，也不把构建日志写进存档。
- 不为并行而放弃确定性排序、完整 manifest 和失败原子性。
- 不因未来可能的 crate 拆分提前制造空 crate 或跨层抽象。
