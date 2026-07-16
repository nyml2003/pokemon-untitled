# 编译产物与任务运行时

> 状态：详细设计，未实现

## 结论

DSL 编译后不解释源文本。编译器以纯流方式将 token/parser 事件转换为 CPS continuation 图；运行时只执行版本化 `ScriptProgram`，并将每个活动脚本保存为可恢复的 `ScriptContinuation`。任务只能在离散世界结算点中断、恢复或提交一个原子操作。

## 编译产物

```rust
pub struct ScriptProgram {
    pub id: ScriptId,
    pub language_version: ScriptLanguageVersion,
    pub bundle: ContentBundleId,
    pub hash: ScriptHash,
    pub continuations: BTreeMap<ContinuationId, CpsNode>,
    pub entry_points: BTreeMap<ScriptEntryId, ContinuationId>,
}

pub enum CpsNode {
    EmitText { text: TextId, arguments: Vec<ProgramValue>, next: ContinuationId },
    RequestChoice { options: Vec<ChoiceBranch> },
    Compare { local: LocalId, op: CompareOp, literal: ProgramValue, yes: ContinuationId, no: ContinuationId },
    RequestOperation { operation: OperationTemplateId, accepted: ContinuationId, rejected: ContinuationId },
    Wait { condition: WaitCondition, resume: ContinuationId },
    Jump { target: ContinuationId },
    End,
}
```

`CpsNode` 是内部 IR，不是稳定手写格式。稳定边界是脚本源、`ScriptLanguageVersion`、资源 catalog 和存档中的 continuation。IR 可以在语言版本内优化，但不得改变可观察语义。

每个外部边界都有显式 continuation：文本展示后继续到 `next`，玩家选择按 option 进入分支，操作按接受/拒绝进入分支，等待条件满足后进入 `resume`。因此中断与恢复不依赖隐藏调用栈或线性 PC 扫描。

## 编译阶段

```text
ASCII ByteStream
  -> incremental tokens
  -> parser events
  -> name/type/effect validation
  -> CPS builder + control-flow validation
  -> ScriptProgram fragments
  -> sealed program manifest
```

编译器是纯 crate。它只依赖传入的 `ByteStream`、资源 catalog、脚本接口 catalog 和 capability catalog，不读文件、不请求模型、不访问世界状态。文件系统导入、bundle 打包和增量缓存属于 adapter 或工具层。

编译器可以按事件输出 `Diagnostic` 与 `ProgramFragment`。外层 adapter 只能在收到成功的 `Finished { manifest }` 后发布产物；发生 error 时不得把部分 fragment 当作可执行脚本。流式不等于接受半个程序。

增量缓存键至少包含：源文件 hash、语言版本、资源 catalog 摘要、脚本接口 catalog 摘要和 capability catalog generation。任一输入变化都会使旧产物失效。

## 运行时状态

```rust
pub struct ScriptContinuation {
    pub instance: ScriptInstanceId,
    pub program: ScriptProgramRef,
    pub next: ContinuationId,
    pub locals: BTreeMap<LocalId, ProgramValue>,
    pub waiting: Option<WaitCondition>,
    pub basis: WorldRevision,
}

pub struct BehaviorTask {
    pub id: TaskId,
    pub actor: WorldActorId,
    pub priority: u16,
    pub continuation: ScriptContinuation,
    pub state: TaskState,
}
```

`ScriptContinuation` 是存档的一部分。它只保存稳定 ID、值和下一个 CPS continuation，不保存 AST 指针、闭包、provider handle、模型请求或 UI 状态。

`WorldRevision` 表示该任务读取世界时的版本。恢复前，调度器必须比较 revision；不匹配时由任务策略决定重读、重试、重新规划或放弃。

## 单步执行

一次 `step` 从 `next` 开始，沿 CPS 节点连续处理纯控制流，直到遇到一个外部可见结果：

```rust
pub enum ScriptStepResult {
    ShowText { text: TextId, next: ScriptContinuation },
    AwaitChoice { options: Vec<ChoiceId>, next: ScriptContinuation },
    AwaitCondition { condition: WaitCondition, next: ScriptContinuation },
    ProposeOperation { proposal: AuthorizedOperationDraft, next: ScriptContinuation },
    Completed,
    Fault(ScriptRuntimeError),
}
```

运行时本身不提交 `AuthorizedOperationDraft`。`world-application` 检查能力、冲突、预算与当前 revision 后，要么原子提交并保留 CPS `next`，要么返回结构化拒绝，让脚本进入 `rejected` continuation、fallback 或结束。

一个结算点最多接受每个 task 的一个世界写入提案。这样操作边界清楚，事件账本可重放，拦截器不会在半个写入中插入。

## 等待与选择

`wait` 只允许等待显式条件：已结算 tick、命名事件、玩家选择、已完成战斗或已提交操作。它不能等待真实时间、网络回调或模型响应。

玩家选择是外部输入。选择时先校验 `ChoiceId` 属于当前 continuation，再将选择结果作为新事件进入下一次结算。UI 只显示 `AwaitChoice`，不能自行推进 CPS 节点。

## 拦截器与任务调度

```rust
pub struct InterceptorBinding {
    pub id: InterceptorId,
    pub scope: InterceptorScope,
    pub priority: u16,
    pub program: ScriptProgramRef,
    pub trigger: TriggerKind,
}

pub enum InterceptorScope {
    Actor(WorldActorId),
    ScriptInstance(ScriptInstanceId),
}
```

结算调度顺序：

1. 收集已唤醒 task、触发的 interceptor 和玩家输入。
2. 丢弃 basis 已过期且策略不允许重试的候选。
3. 按优先级降序、触发类型、稳定 ID 升序排序。
4. 对每个 actor 至多选择一个可运行 task。
5. 运行一步，提交或记录等待结果。
6. 写入 `WorldLedgerEntry`，再发布 `GameEvent`。

新 interceptor 抢占旧任务时，旧任务变为 `Suspended` 并保留 continuation。高优先级任务完成、等待或放弃后，旧任务才可重新参与下一次结算。若 `WorldRevision` 已变化，它不能盲目恢复。

## 存档与迁移

`SaveGameV1` 需要保存：脚本语言版本、内容 bundle、所有非终止 task、continuation、已接受操作、补丁 ID 和账本位置。

加载时按以下顺序处理：

1. 验证 bundle 与脚本版本可用。
2. 验证每个 `ScriptProgramRef`、`ContinuationId` 和局部值仍合法。
3. 执行声明的迁移，或拒绝加载并报告精确版本冲突。
4. 不恢复未完成模型请求；它们必须由确定性策略或下一次结算重新提出。

没有明确定义迁移前，不允许热更新正在运行的脚本。

## 运行时测试

- 相同 bundle、存档、seed、输入和任务顺序生成相同账本。
- 被抢占、等待、选择、provider 拒绝、脚本故障和加载恢复都有 fixture。
- UI tick、渲染帧和模型延迟不会改变世界 task 的结果。
- 无模型、无 GUI、无文件系统的内存 fixture 可以运行完整脚本。
