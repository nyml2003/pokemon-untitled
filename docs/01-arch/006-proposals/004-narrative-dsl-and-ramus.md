# 开放世界叙事脚本 DSL 与 Ramus 能力层提案

> 状态：终态能力提案，未实施
>
> 范围：剧情/行为 DSL、可中断任务、Ramus capability、世界写入审计、AI 内容生成与 GUI/CLI 审计视图。
>
> 前置条件：`WorldState`、事件账本、版本化存档和 `WorldPatch` 的产品合同必须先定义。

> 详细设计：[语言、词法与文法](../../02-narrative-dsl/001-language-and-grammar.md)、[编译产物与任务运行时](../../02-narrative-dsl/002-compiler-and-task-runtime.md)、[Ramus、世界提交与审计](../../02-narrative-dsl/003-ramus-world-commit-and-audit.md)。

## 结论

终态采用双层架构：

```text
叙事与行为 DSL
  -> 编译为 ScriptProgram
  -> 生成受限意图或原子操作请求
  -> Ramus 编译、授权和 provider 调度
  -> 世界应用层验证并提交 WorldPatch / WorldLedgerEntry
```

DSL 负责叙事流程、条件、选择、暂停和恢复。Ramus 负责 capability catalog、参数 schema、principal、授权和 provider 边界。`world-domain` 与 `world-application` 是唯一可以确认世界事实并写入事件账本的所有者。

Ramus 不是世界状态机。DSL 也不是通用程序语言。模型只能生成草案、脚本源或结构化提案，不能获得任意执行权。

## 当前基础

当前 `ramus-core` 已具备下列可复用基础：

- 路径式 `NodePath` 与方法名。
- `Catalog`、参数 schema、schema 版本和 provider ID。
- `Principal`、授权 session、capability generation 与单次 `EffectPermit`。
- `PlanDraft` 到 `TypedPlan` 的密封过程。
- 执行前的 catalog/schema/authorization 检查，以及结构化 provider 错误。

当前这些能力只被 `battle-ramus-adapter` 用于受限战斗动作。它们不拥有世界事件、世界补丁、脚本 continuation、账本或三档业务风险等级。

## 设计目标

1. 编译器以纯流方式读取 ASCII 源，输出 CPS continuation 图；它不需要把完整内容包或 AST 常驻内存。
2. 脚本在构建或闲时预编译。玩家交互期间不依赖模型实时生成脚本。
3. 每个脚本步骤都可验证、可暂停、可恢复、可审计。
4. 所有世界写入都经过同一个产品提交出口，并能产生事件账本记录。
5. 新增 NPC 干扰场景不必改写存量主干脚本。
6. AI 批量生成内容时，错误在解析、编译、静态校验或模拟阶段被拒绝。
7. 同一内容 bundle、存档、seed 和输入序列可重放相同的已结算结果。

## 不把目标误写成现状

以下内容是验收目标，不是当前已证实的事实：

- 严格 LL(1) 文法。
- 纯 ASCII 脚本规范。
- 多线程批量编译吞吐量。
- 权限、查询、补丁和账本的延迟预算。
- 海量 NPC 并发规模。
- GUI 与 CLI 审计工具。

每项性能结论都必须由基准程序和真实世界 fixture 支持。不得把“DSL 很轻”写成纳秒级承诺，或把 provider 的业务成本归因到解析器。

## DSL 的职责与限制

### 负责的内容

- 事件触发、条件、分支和选择。
- NPC 行为任务的启动、挂起、恢复和放弃。
- 对话 text key、脚本调用、等待和受控世界操作请求。
- 将内容层引用连接到 `EventId`、`ScriptId`、`ActorId`、`AnchorId` 等业务 ID。

### 不负责的内容

- 文件、网络、GPU、真实时钟和进程调用。
- 数值计算、战斗伤害、实体扫描或复杂路径搜索。
- 直接持有 `WorldState` 的可变引用。
- 任意循环、递归、动态导入或未受限的反射调用。
- 将自由文本直接解释为生产环境命令。

复杂计算和复合判断放到经过测试的 provider 或领域查询中。DSL 只消费其结构化结果。

## 语法候选

语法目标是规则、可读、容易生成。脚本文件只允许 ASCII。所有玩家可见文案使用外置 i18n key。

```text
#[actor]
actor gate_guard {
  id: actor:gate_guard
}

#[script]
script patrol(actor: actor:gate_guard) {
  move(to: anchor:north_gate)
  wait(until: event:road_clear)
  => script:return_to_post
}

#[intercept(priority: 80)]
intercept guard_danger(actor: actor:gate_guard) {
  if threat_level > 2 {
    /world/actor face(target: ${actor}, direction: dir:south)
    => script:retreat
  } else {
    say(text: text:guard_warning)
  }
}
```

这只是语法草案。它表达的设计取向如下：

- `#[actor]`、`#[script]`、`#[on]`、`#[intercept]` 是元数据标记。
- 参数使用 `name: type = value` 形式；流程操作使用函数调用形式。
- `if / else` 只允许单变量与常量的 `==`、`<`、`>` 比较。
- `${name}` 只引用脚本局部变量。
- `=>` 表示显式转移到另一个脚本或任务。
- 文本使用 `text:` key，不把 UTF-8 文案放入脚本。

在宣称 LL(1) 前，必须为完整文法写递归下降 parser，并用 First/Follow 集、拒绝样例和模糊输入测试证明单 token 前瞻足够。若属性、泛型参数或可选语法糖破坏 LL(1)，应删减语法，而不是引入回溯解析器。

## 符号域与 catalog

脚本词法层按前缀区分符号域：

| 形式 | 含义 | 校验者 |
| --- | --- | --- |
| 无前缀标识符 | DSL 关键字、局部变量、局部标签 | DSL compiler |
| `/path` | Ramus capability 调用 | Ramus catalog + authorization |
| `type:id` | 游戏资源或稳定业务 ID | 内容/世界 catalog |
| `${name}` | 局部变量插值 | ScriptProgram scope |

`type:` 的可用集合必须由版本化 catalog 定义。第一版至少包括 `actor:`、`event:`、`script:`、`anchor:`、`text:` 和 `dir:`。业务 ID 不能退化为任意字符串。

## 行为任务与拦截器

主干脚本表达目标，不枚举所有干扰场景。干扰通过可挂载拦截器注入。

```rust
pub struct BehaviorTask {
    pub id: TaskId,
    pub priority: u16,
    pub program: ScriptProgramId,
    pub continuation: ScriptContinuation,
    pub state: TaskState,
}

pub enum TaskState {
    Running,
    Suspended { by: TaskId },
    Waiting { condition: WaitCondition },
    Abandoned { reason: TaskStopReason },
    Completed,
}
```

拦截器可以挂载到 NPC 全局行为或局部脚本块。世界应用层在结算点收集候选任务，按优先级、适用条件、预算和稳定任务 ID 排序。它随后一次只提交一个确定的任务步骤。

中断规则必须明确：

1. 新任务只能在结算点抢占，不能在一个世界写入操作中间插入。
2. 被抢占任务保存 continuation，或以结构化原因主动放弃。
3. 同优先级冲突由稳定 ID 决定，不能依赖哈希表遍历顺序。
4. 任务恢复前重新检查它的 `WorldRevision`；基础世界已变化时必须重新规划或终止。

这样新增“危险避让”“道路关闭”“角色召唤”等场景时，通常只需新增独立拦截器和 script，不修改主干任务。

## Ramus 与世界提交的边界

Ramus 负责让请求可被授权和执行；它不自动写入 `WorldPatch`。

正确的写入链路如下：

```text
ScriptStep / ModelProposal
  -> PlanDraft
  -> TypedPlan
  -> EffectPermit + ProviderRequest
  -> AuthorizedOperation 或 ProviderError
  -> world-application 验证与原子提交
  -> WorldPatch + WorldLedgerEntry + GameEvent
```

`Provider` 应返回结构化操作或查询结果。`world-application` 根据当前 `WorldState`、内容 bundle、冲突规则和存档策略决定是否接受该操作。这样 provider 不能绕过领域不变量，也不会让 `ramus-core` 依赖游戏业务类型。

终态权限策略在现有 `Read`、`Write`、`Invoke` effect 之外增加产品 policy：

| 风险级别 | 例子 | 默认 principal |
| --- | --- | --- |
| 普通业务 | 对话、局部移动、已定义奖励 | 受限 NPC/script |
| 全局变更 | 改变地图补丁、势力或长期世界旗标 | 审核过的事件 principal |
| 时空高危 | 回滚账本、跨分支迁移、修改世界规则 | 人工工具 principal，AI 默认禁止 |

这三档是上层 policy，不应硬编码进通用 `ramus-core`。Ramus 继续提供 capability 和 permit；游戏定义何种能力属于高危。

## AI 内容与运行时决策

AI 的输出按风险分层：

| 用途 | 输出 | 接受方式 |
| --- | --- | --- |
| 闲时内容制作 | DSL 源、NPC 模板、拦截器、地图补丁草案 | 解析、编译、catalog 校验、模拟与人工审核 |
| 运行时对话 | 已有事件槽位的文案或候选表达 | 不获得世界写入能力 |
| NPC 行为 | `NpcIntent` 或合法任务排序 | 结算点校验后提交 |
| 支线与改图 | `EventProposal` 或 `WorldPatchProposal` | 限额、冲突解决、来源记录后接受 |

模型不直接生成 `TypedPlan`。模型输出先成为不可信草案，再经过编译、schema、权限、预算和世界状态检查。模型失败、超时或输出非法时，规则型 NPC 策略和静态内容必须仍可运行。

## 审计与双视图

CLI 与 GUI 读取同一份不可变数据：`ScriptProgram`、编译诊断、`TypedPlan` 摘要、`WorldLedgerEntry`、`WorldPatch` 和回放结果。

| 视图 | 首要用途 |
| --- | --- |
| CLI | 批量编译、静态校验、模拟、回放、CI 和性能基准 |
| GUI | NPC 模板编辑、流程预览、提案审核、补丁差异和事件回放 |

GUI 不维护另一套脚本或账本格式。它只是同源数据的编辑和可视化入口。

## 验收与基准

### 编译器

- 所有 token、关键字和资源 ID 的 ASCII/前缀规则有正反 fixture。
- 文法测试证明或否定 LL(1) 假设；若否定，简化语法。
- 未解析路径、未知资源、错误参数、越权调用和未受限循环在编译阶段失败。
- 批量编译可并发，但输出、诊断排序和产物 hash 必须确定。

### 运行时

- 相同 world fixture、任务集合、seed 和输入产生相同账本。
- 中断、恢复、放弃和 provider 失败都有可重放测试。
- 运行时无法通过 DSL、Ramus 或模型绕开 `world-application` 的状态提交。
- 存档从 `wait`、选择和被中断任务恢复后，行为与未保存运行一致。

### 性能

基准分别测量：解析、密封计划、授权、只读 provider、写入提案校验、补丁提交、账本写入，以及真实 NPC/world fixture 的端到端结算。报告必须区分纯 DSL 成本、Ramus 成本和具体游戏 provider 成本。

## 分阶段实现

### 阶段 0：固定前置边界

1. 确认唯一权威 `ramus-core` 源码树。
2. 完成 `WorldState`、`WorldPatch`、事件账本和存档合同。
3. 写出 capability catalog 的版本与迁移策略。

### 阶段 1：纯 DSL 编译器

1. 定义最小 AST、资源 catalog 接口和 `ScriptProgram` 格式。
2. 实现 parser、静态校验、确定性诊断和编译 fixture。
3. 先只支持 `say`、`choose`、`wait`、`if`、显式跳转和 `end`。

### 阶段 2：事件执行与任务中断

1. 添加 `ScriptExecutionState`、`BehaviorTask` 和结算调度。
2. 实现局部与全局拦截器、优先级、暂停、恢复和放弃。
3. 在无模型 fixture 中验证回放与存档恢复。

### 阶段 3：Ramus capability 映射

1. 将少量原子操作降低为 `PlanDraft`。
2. 让 provider 返回结构化操作或查询结果。
3. 由 `world-application` 统一提交账本和补丁。

### 阶段 4：审计工具与 AI 流水线

1. 建立 CLI 编译、模拟、回放和基准命令。
2. 建立同源 GUI 审核和流程预览。
3. 接入闲时 AI 草案生成；先审计和模拟，再允许受限运行时决策。

## 当前不做

- 不把 DSL 解释器塞进 UI、host 或 `ramus-core`。
- 不把全文剧情或 UTF-8 自由文案写进脚本源。
- 不允许 AI 直接写存档、地图 JSON、账本或 `TypedPlan`。
- 不为“全局高危”能力默认授予 NPC、脚本或模型 principal。
- 不在没有可恢复执行状态和回放测试前支持任务中断。
- 不在基准完成前承诺具体纳秒、微秒或海量 NPC 吞吐量。
