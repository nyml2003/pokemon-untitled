# Agent 驱动活世界终态提案

> 状态：终态能力提案，未实施
>
> 范围：持久 2D 世界、NPC 意图、支线事件、剧情脚本、受控地图变化与本地模型接入。
>
> 非范围：本提案不授权立即接入模型、不承诺联网，也不定义具体剧情、阵营或数值内容。

## 结论

项目的长期产品单位不是章节，而是持续演化的世界状态。

玩家进入的是一个有历史的 2D 世界。NPC、支线、对话和地图变化都必须从可存档、可验证、可回放的世界事实产生。模型可以提出内容或决策，不能直接修改地图文件、存档或 `GameSession` 内部状态。

终态链路如下：

```text
静态内容 + 已存档世界状态
        |
        v
NPC / 脚本 / 模型提出受限意图
        |
        v
世界规则验证、解决冲突、生成已结算事件
        |
        +--> GameSession 提交产品状态
        +--> WorldPatch 叠加世界变化
        +--> 事件账本与存档
        +--> PresentationState / GameView 显示结果
```

模型不可用、超时或输出非法内容时，世界仍须按确定性规则继续运行。

## 当前基础与缺口

| 项目 | 当前状态 | 终态需要补齐 |
| --- | --- | --- |
| 产品状态 | `GameSession` 已用 `GameCommand`、`GameEvent` 和 snapshot 管理世界与战斗 | 版本化存档、玩家长期状态、已结算世界历史 |
| 地图 | `MapProject` 是版本化静态 JSON | 地图注册表、事件引用、NPC 初始定义和资源 bundle 绑定 |
| 地图事件 | 只有 `MapEventKind::Encounter` | 带 ID、触发器、参数和执行状态的世界事件 |
| NPC | 已有静态 NPC 与统一投影提案 | 运行时状态、意图、关系、日程和冲突解决 |
| Ramus | 只用于受限战斗控制台调用 | 面向脚本原子能力的稳定授权与 provider 边界 |
| 脚本 | 尚不存在 | 可编译、可恢复、可迁移的剧情/事件 DSL |
| 模型 | 尚未接入 | 独立的 inference adapter、结构化提案、限额与 fallback |

`ramus-core` 的唯一权威源码树位于 `crates/foundation/ramus/`，作为根 workspace 的普通 foundation crate。后续扩展通用脚本能力应直接在此 crate 上进行。

## 世界的三层数据

### 1. 静态基线：`MapProject` 与内容 bundle

静态地图是内容源文件。它定义地形、初始人物、交互入口和静态资源引用。运行时不得直接覆盖该 JSON。

静态内容需要引用版本化 bundle：

```rust
pub struct ContentBundleId(/* version + digest */);
pub struct MapId(/* stable world map identity */);
pub struct MapActorId(/* actor identity inside one map */);
pub struct EventId(/* stable interaction and trigger target */);
```

地图中的格子或人物只引用 `EventId`。事件定义、脚本和训练师配置不嵌入格子枚举，也不以裸字符串表示。

### 2. 持久事实：`WorldState`

`WorldState` 是一局游戏的世界真相，属于领域/应用核心，可被版本化存档。它至少包含：

- 当前世界时钟和已结算 tick。
- 玩家所在地图、锚点和位置。
- 每个运行时 NPC 的位置、面向、长期变量和关系摘要。
- 已完成、激活或过期的事件实例。
- 已接受的地图补丁和其生命周期。
- 可重放所需的 seed、规则版本和内容 bundle ID。

`PresentationState` 的动画、对话框开关、输入焦点和真实时间不属于 `WorldState`，也不进入存档。

### 3. 增量变化：`WorldPatch`

运行时地图变化必须表示为叠加在静态基线之上的补丁，而不是原地改写 `MapProject`。

```rust
pub struct WorldPatch {
    pub id: WorldPatchId,
    pub source: WorldPatchSource,
    pub base_bundle: ContentBundleId,
    pub operations: Vec<WorldPatchOperation>,
    pub activation: PatchActivation,
    pub lifetime: PatchLifetime,
}
```

补丁操作只能是有限、可验证的变更，例如添加/移除人物、改变通行性、替换可见物件、挂载或撤销事件。每个操作必须引用已知 ID，且必须通过地图边界、碰撞、资源和事件引用校验。

`WorldPatch` 需要记录来源：人工内容、确定性规则、脚本，或已接受的模型提案。它需要保留在事件账本和存档中。

## 时间与结算

世界使用离散时钟。打开 UI、等待渲染或模型推理都不隐式推进世界。

世界只在明确结算点接收 NPC、脚本或模型结果：

```text
玩家提交动作 / 已到达逻辑 tick / 显式世界事件
        -> 收集可用意图
        -> 验证与排序
        -> 原子提交状态变化
        -> 记录 WorldLedgerEntry
        -> 发布 GameEvent
```

同一份 `WorldState`、同一组意图和同一 seed 必须得到相同结果。模型结果到达过晚时，本 tick 不回滚；它只能进入后续结算点或被丢弃。

## NPC 与 Agent 边界

NPC 不是一个每帧调用模型的对象。NPC 由持久状态、明确意图和规则调度组成。

```rust
pub struct NpcIntent {
    pub actor: WorldActorId,
    pub basis: WorldRevision,
    pub action: NpcAction,
}

pub enum NpcAction {
    Move { direction: Direction },
    Face { direction: Direction },
    Interact { event: EventId },
    ProposeEvent { proposal: EventProposalId },
    Wait,
}
```

模型可以根据经过裁剪的 NPC 上下文提出 `NpcIntent` 或 `EventProposal`。模型不能获得完整存档、其他 NPC 的私有状态、文件路径或任意系统能力。世界规则负责检查位置、权限、冷却、可见性、冲突和预算。

第一版 NPC 决策必须有确定性策略作为 fallback。模型只替代部分规划，不成为世界继续运行的前提。

## 剧情与事件 DSL

剧情 DSL 位于 Ramus 之上。它面向内容作者，表达流程、条件、选择、暂停和恢复；它不是通用 Rust、文件系统脚本或模型提示词。语法、任务中断和 Ramus 映射的详细方案见[开放世界叙事脚本 DSL 与 Ramus 能力层提案](004-narrative-dsl-and-ramus.md)。

最小指令集应覆盖：

- `say`：显示带参数的文本 key。
- `choose`：等待玩家或规则选择。
- `when`：检查已公开的世界条件。
- `set_flag`、`grant_item`、`start_battle`、`warp`：提出受控世界操作。
- `wait`：保存 continuation，等待明确事件或逻辑 tick。
- `end`：结束事件实例。

脚本源文件先编译为版本化 `ScriptProgram`。运行时只执行已编译程序，并把可恢复位置保存为 `ScriptExecutionState`。不得允许脚本读取文件、网络、GPU、真实时钟，或产生无界递归/循环。

```text
ScriptProgram
  -> ScriptExecutionState + WorldState
  -> ScriptStepResult
  -> authorized atomic operations
  -> WorldState / GameSession transition
```

脚本热更新必须有版本策略：拒绝正在执行的旧版本、提供迁移，或把旧 bundle 随存档保留。不能在加载旧存档时静默以新语义解释旧指令。

## Ramus 的位置

Ramus 保持为通用的能力、授权、编译计划和 provider 执行边界。它不拥有地图、NPC、任务或剧情状态。

剧情 DSL 的一个原子操作可以降低为受授权 capability，但不要求作者写 Ramus 的路径式命令。模型也不应直接输出 Ramus 文本。

```text
内容作者 / 模型
  -> ScriptProgram 或结构化提案
  -> 领域验证
  -> Ramus capability / typed command
  -> provider
  -> 结构化执行结果
```

脚本 principal、能力集合、单次操作预算和 provider 错误都必须有明确的版本和错误模型。

## 模型接入

模型是 adapter。核心只依赖一个由产品需要定义的端口：

```rust
pub trait InferencePort {
    fn propose(
        &self,
        request: ModelDecisionRequest,
    ) -> Result<ModelProposal, InferenceFailure>;
}
```

`ModelDecisionRequest` 需要包含 schema 版本、经裁剪的世界事实、允许的 proposal 类型、输出大小上限、超时、上下文预算和 seed。`ModelProposal` 必须是结构化数据，不是自由文本命令。

模型能力按风险递进：

| 阶段 | 模型职责 | 提交方式 |
| --- | --- | --- |
| 内容制作 | 生成地图、NPC、对话和脚本草案 | 编辑器编译、校验、人工接受后写入内容 bundle |
| 运行时对话 | 填充由脚本决定的对话表达 | 文本不能直接改变世界事实 |
| NPC 决策 | 在合法意图中排序或选择 | 规则引擎验证后在结算点提交 |
| 支线提案 | 提出带预算的 `EventProposal` | 编译、模拟、校验后变为事件实例 |
| 地图变化 | 提出有限 `WorldPatchProposal` | 校验、冲突解决、记录来源后接受 |

任何阶段都必须支持 `Timeout`、`Unavailable`、`MalformedProposal`、`RejectedByPolicy` 和 `BudgetExceeded`。这些是正常失败，应由确定性 fallback 处理。

## 目标边界

| 层 | 终态职责 | 不负责 |
| --- | --- | --- |
| `map-project` | 静态地图、人物初始定义、事件引用、格式迁移 | 运行时 NPC 状态、模型调用 |
| `world-domain` | 格子规则、人物阻挡、世界事实、补丁验证 | 文字渲染、文件和 GPU |
| `world-application` | 世界 tick、意图编排、事件与补丁提交 | 窗口时钟、模型后端细节 |
| `game-session` | 玩家命令、场景、世界/战斗生命周期 | 脚本语法、模型 prompt |
| `world-event-domain` | 事件、脚本程序、continuation 和迁移 | 地图资源、输入设备 |
| `npc-domain` | NPC 关系、日程和可复用决策规则 | 地图文件、渲染帧 |
| Ramus adapter | capability、provider、授权与执行诊断 | 剧情流程和世界真相 |
| model adapter | 本地/远程推理、限流和结果解码 | 规则判断和状态提交 |
| presentation | 显示快照、对话、动画和输入 | 直接改写世界状态 |

`world-event-domain` 与 `npc-domain` 不是立即要创建的 crate。只有在它们拥有独立不变量，并被至少两个上层消费者使用时才拆出；此前应优先扩展现有 `world-domain` 与 `world-application`。

## 分阶段实现

### 阶段 0：收口现有基线

1. 修复格式、lint 和资源验证，使集成基线可重复通过。
2. 明确单机、本地模型和未来联网的产品边界。

完成标准：当前 workspace 的质量门禁可重复运行；没有依赖未构建 Ramus 副本的改动。

### 阶段 1：多人世界基础

按 `002-world-characters-and-npc-contract.md` 实现静态 NPC、多人阻挡、统一人物快照和交互 ID。

完成标准：无模型 NPC 可显示、阻挡、被交互；主角移动和战斗不回归。

### 阶段 2：世界事件与存档

1. 用 `EventId` 和触发器替代单一 `MapEventKind::Encounter`。
2. 定义版本化 `SaveGameV1`、`WorldState` 和 `WorldLedgerEntry`。
3. 定义 `WorldPatch` 与内容 bundle 的兼容规则。

完成标准：加载同一存档后，世界位置、NPC 状态、已结算事件和地图补丁完全一致。

### 阶段 3：无模型事件 DSL

1. 实现最小 `ScriptProgram`、编译器和可恢复执行状态。
2. 跑通对话、选择、奖励、传送和战斗入口。
3. 将原子操作接到受授权的 capability/provider。

完成标准：脚本无法绕过世界验证；脚本在任意 `wait` 或选择处存档后可恢复。

### 阶段 4：确定性 NPC 模拟

1. 用规则策略生成 `NpcIntent`。
2. 在显式结算点解决移动、交互和事件冲突。
3. 为同一 fixture 锁定世界演化事件序列。

完成标准：没有模型服务时，世界仍可持续运行且结果可复放。

### 阶段 5：模型提案 adapter

1. 先接入编辑器内的离线内容草案生成。
2. 再接运行时对话和合法 NPC 意图排序。
3. 最后试验支线和地图补丁提案。

完成标准：模型输出非法、超时或不可用时，不会损坏存档、阻塞世界结算或产生未校验状态。

## 验收规则

- UI、模型和 provider 都不能直接修改 `WorldState` 或 `GameSession`。
- 每个已接受的模型提案、脚本步骤和地图补丁都有稳定 ID、版本和来源记录。
- 同一静态 bundle、存档、seed 和输入序列可重放同一组已结算事件。
- 运行时地图变化只通过已验证的 `WorldPatch` 发生。
- 脚本与模型失败有结构化结果和确定性 fallback。
- 存档不保存窗口、GPU、动画、输入焦点或未结算的模型请求。
- 任何新增 capability 都有权限、参数、预算和失败路径测试。

## 当前不做

- 不让模型直接写 `maps/*.json`、存档或 Rust 源码。
- 不让每个 NPC 每帧调用模型。
- 不用自由文本作为生产环境的执行命令格式。
- 不在模型推理完成前阻塞玩家移动或世界结算。
- 不在没有存档、事件账本和补丁校验前实现动态改图。
- 不因终态设想提前把所有系统拆成新 crate。
