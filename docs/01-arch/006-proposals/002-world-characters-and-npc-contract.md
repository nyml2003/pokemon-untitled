# 世界人物与 NPC 契约方案

> 状态：提案，未实施
>
> 范围：主角与 NPC 的地图位置、朝向、移动能力、精灵资源和交互入口。不包含对话内容、任务、存档、脚本语言或战斗规则。

## 结论

第一阶段不新建 crate。

现有 `world-domain` 已经拥有格子位置、朝向、通行和移动规则。`map-project` 已经拥有可编辑、可序列化的地图内容。现在新建 `character-domain` 会让人物位置和世界碰撞规则分在两个纯 crate，随后 `world-domain` 为了阻挡和交互又必须依赖它，边界会变差。

第一阶段在现有链上增加一个统一的“世界人物投影契约”。它只表达所有可见人物共有的事实：身份、位置、朝向、外观和当前移动语义。主角与 NPC 的差异用能力和专属状态表达，不让一个巨大的 `Character` 结构同时承载输入、队伍、剧情、脚本和渲染细节。

```text
MapProject 的静态 NPC 定义
        |
        v
world-application 构造并推进世界人物
        |
        v
GameSession 发布 WorldSnapshot / WorldEvent
        |
        v
PresentationState 维护每个人物的短暂动画进度
        |
        v
game-view 投影人物图像
        |
        v
game-asset-plan 只请求当前地图实际使用的精灵集
```

## 当前事实

- `world-domain::World` 只保存一个玩家的位置和朝向。`WorldCommand` 只支持面向和移动。
- `world-application::WorldObservation` 只暴露 `player` 与 `facing`。
- `game-session` 的 `GameCommand::StepWorld` 只驱动玩家，遭遇事件通过 `WorldEvent` 进入战斗。
- `game-ui::PresentationState` 拥有主角的 `Stand`、`Walk`、`Run`、`RunStopping` 表现状态和插值。
- `game-view` 只在角色层投影一个主角图像；`game-asset-plan` 将固定的逻辑资源键绑定到 `character/red/...`。
- 新导入的 `assets/source/character/dppt/000...087` 每个角色只有四方向的站立和步行帧。它们适合 NPC，不能假设具有跑步帧。
- `MapProject` 还没有人物列表、人物 ID、阻挡占格或交互定义。

这说明第一件事不是扩展 `WorldAnimation`，而是使世界快照和资源计划能够表达多个、动作能力不同的人物。

## 统一契约

### 契约的范围

统一契约是值类型，不是 Rust trait。它用于领域快照、表现投影和资产解析之间传递数据。

```rust
pub struct WorldActorSnapshot {
    pub id: WorldActorId,
    pub role: WorldActorRole,
    pub position: Position,
    pub facing: Direction,
    pub appearance: CharacterAppearanceId,
    pub motion: WorldMotionKind,
}

pub enum WorldActorRole {
    Player,
    Npc,
}

pub enum WorldMotionKind {
    Stand,
    Walk,
    Run,
    RunStopping,
}
```

`WorldActorId` 与 `CharacterAppearanceId` 都必须是 newtype。前者表示运行时人物实例，后者表示一套静态精灵资源。不能以裸字符串或资产键代替其中任意一个。

这个快照不包含下列字段：

- 键盘输入、AI tick、计时器或像素偏移。
- 角色名、对话文本、任务进度或脚本句柄。
- 主角队伍、背包、徽章、金钱或战斗状态。
- 文件路径、PNG 文件名、图集坐标或 GPU 资源 ID。

这些数据的变化速度、所有者和持久化语义都不同。将它们合并会让 NPC 为主角专属状态付出复杂度，也会让表现层控制游戏规则。

### 能力与当前动作分开

人物当前在做什么，与其精灵集能显示什么，是两个概念。

```rust
pub struct CharacterMotionSet {
    pub stand: bool,
    pub walk: bool,
    pub run: bool,
    pub run_stopping: bool,
}
```

第一版不直接把这个结构放进每帧快照。它属于 `CharacterAppearanceId` 的静态定义，供资源计划和校验使用。

| 人物 | 允许的 `WorldMotionKind` | 精灵集 |
| --- | --- | --- |
| 主角 | `Stand`、`Walk`、`Run`、`RunStopping` | 现有 `character/red/...` |
| NPC | `Stand`、`Walk` | `character/dppt/<编号>/...` |

当请求的动作不被精灵集支持时，领域层不得静默降级。构造地图或加载资源时应返回结构化错误。表现层可在开发模式额外显示缺失资源诊断，但不能决定规则上的替代动作。

## 所有权与边界

| 数据或规则 | 所有者 | 说明 |
| --- | --- | --- |
| `MapActorId`、NPC 初始位置、朝向、外观、阻挡和交互引用 | `map-project` | 静态、可编辑、可序列化的地图内容。 |
| 人物位置、朝向、占格、移动合法性、人物间阻挡 | `world-domain` | 纯格子规则。不读地图文件，也不看 PNG。 |
| 地图项目到世界人物的转换、NPC 移动编排 | `world-application` | 把地图定义转为运行时世界，协调纯领域操作。 |
| 玩家输入、场景切换、与 NPC 交互的产品事件 | `game-session` | 只对玩家暴露产品命令。 |
| 插值、走路帧循环、跑步收步、临时转向展示 | `game-ui` | 不保存人物位置真相，也不改变世界规则。 |
| 多人物投影、层级与资源键选择 | `game-view` | 从快照和表现状态生成 `ViewImage`。 |
| 外观定义到 `AssetKey` 的映射、按地图请求精灵资源 | `game-asset-plan` | 不能把资源目录名泄露到领域。 |

## 地图数据草案

`MapProject` 增加一个 `actors` 字段。格式版本应从 `gen3-map-v1` 升到明确的新版本；不要让旧地图在反序列化时悄悄得到默认 NPC 语义。

```rust
pub struct MapActor {
    pub id: MapActorId,
    pub role: MapActorRole,
    pub position: TilePosition,
    pub facing: MapDirection,
    pub appearance: CharacterAppearanceId,
    pub collision: ActorCollision,
    pub interaction: Option<MapInteractionId>,
}

pub enum MapActorRole {
    Npc,
}

pub enum ActorCollision {
    BlocksMovement,
    PassThrough,
}
```

主角不作为 `actors` 的一项持久化。它继续由 `player_spawn` 和产品存档决定，避免同一人物有两个初始位置来源。

验证规则：

1. `MapActorId` 在同一张地图内唯一。
2. NPC 坐标在地图范围内，且位于可通行格。
3. `BlocksMovement` NPC 不得与主角出生点或另一个阻挡 NPC 重叠。
4. `CharacterAppearanceId` 必须能解析为已知外观，且其动作集至少支持 `Stand` 与 `Walk`。
5. `interaction` 为空时，面对 NPC 只能得到“无可执行交互”；不以空字符串或虚假事件代替。

第一版只放置静态 NPC。巡逻、转向、跟随和脚本移动必须在静态人物、阻挡和交互测试通过后再增加。

## 运行时规则草案

`world-domain` 的世界状态从单个 `player` 演进为一个明确的玩家 ID 和人物集合。集合内部不直接暴露可变引用。

```rust
pub struct World {
    map: TileMap,
    player_id: WorldActorId,
    actors: BTreeMap<WorldActorId, WorldActor>,
}

pub enum WorldCommand {
    FacePlayer(Direction),
    MovePlayer { direction: Direction, pace: PlayerPace },
    MoveNpc { actor: WorldActorId, direction: Direction },
}

pub enum PlayerPace {
    Walk,
    Run,
}
```

`MoveNpc` 不是公开给 UI 的产品命令。它只由 `world-application` 的 NPC 行为编排调用。第一阶段不实现 NPC 行为编排，可先不公开该分支。

人物间阻挡必须与瓦片阻挡在同一个领域操作中判断。移动目标被 NPC 占据时，世界状态不变，但玩家仍朝向目标格。输出 `WorldEvent::BlockedByActor { actor, at }`，而不是把 NPC 当作 `Tile::Wall` 丢失身份。

玩家面对相邻 NPC 并提交 `GameCommand::Interact` 时，`world-domain` 返回 `WorldEvent::InteractionAvailable { actor, interaction }`。`game-session` 再决定它是打开对话、发起训练师战斗还是无事发生。世界领域不加载文本，不创建战斗，也不读取任务状态。

## 表现与资产草案

### 表现状态

`PresentationState` 改为按 `WorldActorId` 保存表现状态：

```rust
pub struct ActorPresentation {
    pub motion: WorldMotionKind,
    pub sprite_frame: usize,
    pub pixel_offset: PixelOffset,
}
```

主角继续使用现有的移动插值和 `RunStopping`。静态 NPC 固定为 `Stand`，所以没有额外计时器。未来会移动的 NPC 通过收到的 `WorldEvent::Moved` 创建一个 `Walk` 插值；不从渲染帧率或 AI 计时器推导位置。

### 资源解析

`game-view` 不再生成 `character/<方向索引>/<帧索引>` 这样的全局资源键。它生成包含 `WorldActorId`、动作和帧的逻辑请求，资产计划把请求映射到外观实际提供的 `AssetKey`。

```text
WorldActorSnapshot { appearance: dppt/042, facing: Left, motion: Walk }
    -> CharacterFrameRequest { actor, appearance, direction, motion, frame }
    -> AssetKey("character/dppt/042/left/walk/01")
```

这样同一帧可以有多个角色使用不同精灵集，且不会加载全部 88 个 NPC 的 1,056 张图片。地图加载或场景切换时，资产计划只为当前地图中的外观加上主角外观发出请求。

## 不做的方案

### 不以 `trait Character` 统一主角与 NPC

trait 会迫使调用方猜测哪些方法可用，或为 NPC 提供没有意义的队伍、输入和任务 API。快照值类型加上角色专属聚合，能让数据流和能力边界更清楚。

### 不让 `WorldAnimation` 成为领域规则

`WorldAnimation` 现在属于 `game-ui`。跑步收步和帧循环是表现细节，不能用它判断人物是否可以移动、阻挡或触发事件。领域只表达移动速度语义和已发生的动作。

### 不在第一阶段新建 `character-domain`

人物的空间规则离不开 `Position`、`Direction`、地图边界和占格。现阶段拆包会引入循环依赖或重复类型。先让 `world-domain` 拥有这些规则；当出现独立且可复用的非空间人物业务时再拆分。

## 何时新增 crate

以下任一条件成立时，再新建 crate，并以具体消费者验证它的 API：

| 条件 | 建议 crate | 不属于它的内容 |
| --- | --- | --- |
| 主角有持久化的队伍外状态、旅行能力、徽章、背包或金钱，且同时被世界、战斗入口和菜单使用 | `player-domain` | 格子碰撞、NPC 位置、UI 焦点。 |
| NPC 需要日程、巡逻、关系、可复用决策规则，并被多个地图或模拟入口调用 | `npc-domain` | 地图文件读写、渲染帧、对话 UI。 |
| 对话、训练师战斗、传送和脚本需要统一的可恢复执行状态 | `world-event-domain` | 人物精灵、键盘输入、GPU 资源。 |

新增 crate 前必须证明：它有独立的不变量；至少两个上层消费者需要它；它不会复制 `Position`、`Direction` 或 `AssetKey`；并且可以用内存 fixture 测试。

## 分阶段实施

### 阶段 1：静态 NPC 和统一投影

1. 为 `map-project` 增加 `MapActor`、地图版本迁移和验证测试。
2. 在 `world-domain` 增加人物 ID、人物集合、人物阻挡和多人物快照。
3. 在 `world-application` 从 `MapProject` 建立 NPC，并保持现有地图无 NPC 时的行为不变。
4. 扩展 `GameSnapshot`、`game-view` 和 `game-ui`，使角色层可以投影主角和静态 NPC。
5. 将 `game-asset-plan` 从固定 `red` 表改为“当前场景外观集合”的请求计划。
6. 把一名 `dppt` NPC 放入 demo map，验证四方向站立、玩家阻挡和资源加载。

完成标准：一个没有交互的 NPC 可显示、阻挡玩家；现有主角跑步和野外遭遇不回归；未使用的 DPPT 外观不会被请求或放入图集。

### 阶段 2：交互

1. 增加 `GameCommand::Interact` 与面向目标解析。
2. 发布带 `MapActorId` 的 `WorldEvent::InteractionAvailable`。
3. 在 `game-session` 选择产品结果；最小版本只显示一个稳定交互 ID，不引入文本系统。
4. 在 `PresentationState` 和 UI 中添加交互覆盖层，不直接修改世界状态。

完成标准：面对 NPC 并交互时，只有指定事件发生；空交互、非相邻目标和被阻挡的目标都有确定结果。

### 阶段 3：受控 NPC 移动

1. 在 `world-application` 定义输入为显式的 NPC 意图或脚本结果，而不是时钟回调。
2. 复用人物阻挡规则，支持 NPC 走路和转向。
3. `game-ui` 按领域事件创建每个 NPC 的步行动画。

完成标准：给定同一组 NPC 意图和地图，位置、事件和表现帧序列完全确定；NPC 永远不会跑步或请求缺失动作帧。

## 测试与验收

| 层级 | 重点 |
| --- | --- |
| `map-project` | 人物 ID、越界、重叠、阻挡格、外观与地图格式迁移。 |
| `world-domain` | 玩家/NPC/瓦片阻挡、转向、相邻交互目标、事件顺序。 |
| `world-application` | 项目到运行时人物的转换，以及无 NPC 的兼容行为。 |
| `game-session` | 只允许玩家输入推进世界；交互事件不会由 UI 直接生成。 |
| `game-ui` | 主角与 NPC 动画状态独立；世界快照不随表现 tick 改变。 |
| `game-view` | 人物层包含稳定顺序的多张图像，且每张图像请求正确的逻辑外观。 |
| `game-asset-plan` | 仅请求主角和当前地图人物的动作集；NPC 不请求跑步资源。 |
| 真实运行 | 主角可绕行和碰撞 NPC，NPC 在地图与镜头移动时保持正确位置，切换地图不残留旧人物。 |

每完成一个阶段，先运行涉及 crate 的测试；阶段完成时运行 `cargo test --workspace`。资源计划还必须通过 catalog 完整性检查和一次真实 native target 图集加载。

## 待定决策

1. NPC 是否允许 `PassThrough`，以及它是否仅服务剧情演出。
2. 主角跑步是否受地图、地形、剧情或旅行能力限制。
3. `MapInteractionId` 是直接引用文本/训练师定义，还是引用将来的世界事件定义。
4. 地图人物的稳定存档键是 `MapActorId`，还是 `MapId + MapActorId` 的组合。
5. 多人物同格时的显示排序规则。第一版建议按脚底行、列、再按 `WorldActorId` 排序。
