# Ramus、世界提交与审计

> 状态：详细设计，未实现

## 结论

Ramus 是能力授权中间层，不是世界写入器。它把不可信草案密封为 `TypedPlan`，并让 provider 返回结构化查询或操作提案。只有 `world-application` 能把提案提交为 `WorldPatch`、`WorldLedgerEntry` 和 `GameEvent`。

## Capability catalog

每个 capability 注册项需要稳定路径、方法、参数 schema、schema 版本、effect、provider 和产品 policy 标签。

```text
/world/actor face(target: actor, direction: dir) -> Invoke
/world/event start(event: event) -> Invoke
/world/inventory grant(item: item, amount: integer) -> Write
/world/query actor_state(target: actor) -> Read
```

Ramus 已有 `Read`、`Write`、`Invoke` effect 与 schema 版本。产品层额外定义风险 policy：

| policy | 允许的例子 | 默认调用者 |
| --- | --- | --- |
| `Local` | 对话、面向、已定义奖励 | 受限 script/NPC |
| `World` | 长期旗标、地图补丁、势力变化 | 审核事件 principal |
| `Temporal` | 账本回滚、跨分支、规则替换 | 人工审计工具 |

模型、普通 NPC 和内容脚本默认没有 `Temporal` 权限。`World` 权限也必须在 capability grant 与世界预算两层同时允许。

## 密封与提交

```text
ScriptProgram / ModelProposal
  -> PlanDraft
  -> compiler seals TypedPlan
  -> Ramus preflight + EffectPermit
  -> ProviderRequest
  -> QueryResult | AuthorizedOperationDraft | ProviderError
  -> world-application resolve
  -> WorldPatch / WorldLedgerEntry / GameEvent
```

`TypedPlan` 不是存档格式，不应被模型直接生成。它绑定 catalog generation、schema version、principal 和 capability generation；任一方变化后，计划必须重新密封。

`AuthorizedOperationDraft` 至少包含 operation ID、目标资源、参数、来源 principal、basis `WorldRevision`、预算类别和可选幂等键。它不包含对 `WorldState` 的可变引用。

## 原子提交

世界应用层按以下规则处理写入提案：

1. 检查 proposal 对应的 bundle、世界 revision 和 capability policy。
2. 读取所需领域事实，验证业务不变量与冲突。
3. 生成零或一个 `WorldPatch` 与零或多个领域事件。
4. 原子更新 `WorldState`、账本位置和 task continuation。
5. 发布 `GameEvent` 给表现层。

任一步失败都不应部分写入世界。失败需返回 `UnknownResource`、`StaleRevision`、`PolicyDenied`、`BudgetExceeded`、`Conflict` 或 `ProviderRejected` 等结构化结果。

## 账本

账本是可重放事实记录，不是调试日志。

```rust
pub struct WorldLedgerEntry {
    pub id: LedgerEntryId,
    pub tick: WorldTick,
    pub source: LedgerSource,
    pub basis: WorldRevision,
    pub operation: OperationId,
    pub patch: Option<WorldPatchId>,
    pub outcome: LedgerOutcome,
}
```

来源至少区分玩家、静态脚本、规则型 NPC、模型提案和人工工具。账本记录已接受操作和必要的拒绝摘要；它不应保存完整 prompt、私密上下文或 GPU 运行细节。

## AI 提案

AI 输出先解码为版本化 `ModelProposal`。可接受类型限于对话草案、`NpcIntent`、`EventProposal` 和 `WorldPatchProposal`。

每个请求带上：schema 版本、可见世界摘要、允许 proposal 类型、最大输出大小、deadline、seed、budget 和来源模型版本。每个接受结果带上 proposal ID、输入摘要和接受/拒绝原因。

模型超时、不可用或格式错误时，调度器必须使用规则型 fallback 或跳过本次决策。世界不能等待模型而停止结算。

## CLI 与 GUI

CLI 和 GUI 必须读取同一套 bundle、编译诊断、计划摘要、账本、补丁与 replay 数据。

| 工具 | 第一版命令或视图 |
| --- | --- |
| CLI | `check`、`compile`、`simulate`、`replay`、`benchmark` |
| GUI | 脚本流程图、NPC task 状态、补丁 diff、proposal 审核、账本时间线 |

GUI 不能通过另一套私有格式保存脚本或账本。编辑器提交的内容也必须经过同一个 compiler 和 catalog 校验。

## 审计与安全测试

- 改变 catalog、schema、principal grant 或 bundle 后，旧 `TypedPlan` 被拒绝。
- provider 无法绕过世界提交出口。
- 每个 `WorldPatch` 都可追溯到 bundle、script/task、principal 或模型 proposal。
- replay 不依赖 GUI、真实时钟、网络或模型服务。
- AI 无法请求未在 catalog 声明的能力，无法构造 `TypedPlan`，也无法取得 `Temporal` policy。
- 基准分别报告 parser、seal、authorization、provider、世界提交和账本写入成本。
