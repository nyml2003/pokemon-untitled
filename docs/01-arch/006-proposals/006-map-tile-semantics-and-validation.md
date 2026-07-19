# 地图瓦片语义目录与硬校验方案

> 状态：第一期实施方案

## 结论

第一期新增独立的瓦片语义目录和纯 Rust 校验器。它不改变 `MapProject` 的 JSON 格式，不改变 `CompositeTile.layers` 的绘制顺序，也不要求编辑器立刻改用新画笔。

目录为每个原子瓦片记录可否使用、同格图层要求和八邻域硬要求。校验器逐格、逐层检查地图。它不会把一个格子误当成单一瓦片：`[草地, 树 A, 树 B]` 中的两片树都会独立检查自己的下方与邻域规则。

未确认的瓦片不能被模型或地图使用。第一期先批准当前正式地图引用的瓦片，其余瓦片保留目录条目但标为禁止使用。

## 范围

第一期处理以下硬错误：

- 地图使用了没有目录条目或标为禁止的原子瓦片。
- 地表、树片、草丛等瓦片放在错误的同格图层位置。
- 3x3 树、1x2 灌木等图样缺少必要的相邻部件。
- 相邻格没有满足已声明的瓦片、图样部件或标签条件。
- 多层叠加中出现未声明的顺序或组合。

第一期不处理推荐搭配、自动拼图、自动修复、地图生成策略，也不把碰撞和事件层绑定到视觉语义。碰撞和事件继续使用现有平行数组。

## 数据模型

新增纯领域 crate `crates/domain/map/map-tile-semantics`。它依赖 `map-project`，不读取文件、不依赖窗口、GPU 或 CLI。

核心类型如下：

```rust
pub struct TileHardRules {
    pub stack: Vec<StackRule>,
    pub neighbours: Neighbours8<NeighbourRule>,
}

pub enum TileMatcher {
    AtomicTile(AtomicTileId),
    Tagged(TileTag),
    PatternPart { pattern: PatternId, part: PatternCoord },
    AnyOf(Vec<TileMatcher>),
}

pub enum NeighbourRule {
    Any,
    Requires(CellRequirement),
    Forbids(CellRequirement),
}
```

`Neighbours8` 固定保存上、右上、右、右下、下、左下、左、左上八个方向。每个方向必须存在；无约束时显式写 `Any`。

`CellRequirement` 同时包含 `TileMatcher` 与图层范围。范围为底层、顶层或任意层。`AtomicTile` 默认匹配指定原子瓦片是否出现在目标范围内，不要求邻格只有这一层。

`StackRule` 至少支持：

- `MustBeBase`：瓦片必须是格子的底层。
- `RequiresBelow(TileMatcher)`：当前瓦片下方必须存在满足条件的图层。

图样定义保存完整部件网格。部件声明自身的 `PatternId + PatternCoord`。校验器据此生成该部件在八邻域中必须出现的相邻部件条件。图样有空洞时，空洞不默认要求相邻格为空；需要禁止叠加时，目录显式使用 `Forbids`。

## 语义目录

新增 `assets/source/map/tile/tile-semantics-v1.json`。它是内容数据，不属于 Rust crate。

目录必须和资产加载出的原子瓦片集合完全一致。当前 tileset 有 292 个唯一原子瓦片，因此 JSON 必须有 292 个唯一条目。

每个条目为以下两种状态之一：

```json
{
  "id": "tile-0020",
  "status": "approved",
  "tags": ["temperate-tree"],
  "rules": {}
}
```

```json
{
  "id": "tile-0122",
  "status": "blocked",
  "reason": "not reviewed for map authoring"
}
```

第一期批准 `maps/demo-map.json` 和 `maps/verdant-route/` 实际引用且已完成约束编目的 45 个瓦片。其余 247 个条目必须存在，但保持 `blocked`。这保证模型不能猜测未知素材的用途。仅出现在未使用材料定义、但尚未形成完整图样或层规则的旧瓦片同样保持 `blocked`。

现有 `semantic-hints.json` 是范围猜测和自然语言说明，只能作为后续人工编目的研究输入。加载器和校验器绝不读取它；`tile-semantics-v1.json` 是唯一权威目录。

### 本次首批目录

- 292 个原子瓦片均有目录条目；45 个已编目正式地图瓦片为 `approved`，其余为 `blocked`。
- `tile-0102` 标为 `meadow` 且必须在底层。树片、高草和草地覆盖物要求其下方存在 `meadow`；`tile-0262` 与 `tile-0263` 还强制组成完整的左右 1x2 林下植被。沙地和水面必须位于底层，睡莲必须位于水面之上。
- 八邻域字段已完整建模并进入校验器。生产目录已启用人工逐图确认的 `temperate-tree-003`、`temperate-tree-001` 和 `tall-grass-015` 图样；它们分别强制完整 3x3、2x3 和 4x5 邻域，不能再被拆散摆放。
- 领域测试已覆盖完整图样、任意缺片、标签、图样部件、八方向和多层图层栈；完成审核后，只需把已确认的 `patterns` 填入权威目录即可开始对生产地图强制执行。

## 校验流程

`MapProject::validate` 继续只检查结构：尺寸、引用、已知原子瓦片、出生点和人物。`map-tile-semantics` 在结构校验成功后执行。

校验器按以下顺序工作：

1. 建立每个格子的有序原子瓦片栈。
2. 遍历每一层原子瓦片，确认目录条目存在且已批准。
3. 对当前层检查 `StackRule`。
4. 对八个方向读取相邻格图层栈，检查 `NeighbourRule`。
5. 收集全部 `MapSemanticDiagnostic`，不在第一个错误处停止。

诊断必须包含地图坐标、当前原子瓦片、其层序号、规则位置（同格或方向）、期望条件和实际图层栈。模型和人工编辑者必须能据此定位和修正错误。

## 接入边界

```text
assets/source/map/tile/tile-semantics-v1.json
        |
        v
runtime adapter parses JSON
        |
        v
map-tile-semantics -- validates --> MapProject
        ^                                  ^
        |                                  |
map-editor-cli / map-editor            game-host
```

目录解析由 runtime 或 adapter 完成，纯 crate 只接收已解析的数据。

- `map-editor-cli` 新增 `lint <map-or-directory> --json`。它加载 JSON 或 `.g3mp` 地图，执行结构与语义校验，并输出稳定的机器可读诊断。
- JSON Lines 编辑接口新增只读语义校验命令。模型可在保存前获得全部错误。
- `map-editor` 可以打开不合规地图以便修复，但保存前必须通过语义校验。
- `game-host` 在加载每张区域地图时执行同一校验，禁止不合规内容进入游戏。
- `CreateMaterial`、`AppendAtomicLayer` 和 `PaintVisual` 暂时保留。第二期再为模型新增完整图样和语义摆放命令。

## 测试与完成标准

纯 crate 测试至少覆盖：

1. 目录与 292 个已知原子瓦片完全对应。
2. 被禁止或缺失的瓦片产生诊断。
3. 完整 3x3 树通过；任意缺片都报告对应方向。
4. 1x2 灌木缺少下半块时失败。
5. 合法 `[地表, 树 A, 树 B]` 多层叠加通过；错误顺序失败。
6. `AtomicTile`、标签、图样部件和 `AnyOf` 的匹配行为。
7. 地图边缘的缺失邻格诊断。

集成验收：

- `map-editor-cli lint maps/demo-map.json --json` 通过。
- `map-editor-cli lint maps/verdant-route --json` 通过。
- 编辑器保存和 `game-host` 加载会拒绝同一份故意破坏的地图。
- 新 crate 纳入 `scripts/test_pure_coverage.py` 的 100% 生产行覆盖率门禁。
