# UI 框架升级报告

> 日期：2026-07-19
> 状态：P0 核心 API、P1 基础组件、图鉴和战斗主菜单/招式详情迁移已完成；其他页面与编辑器仍未迁移
> 范围：`punctum-ui`、游戏页面投影、地图编辑器与瓦片编辑器的 UI 接线。

## 结论

当前 UI 框架的主要问题不是 Flex 能力不足，而是低层布局细节直接暴露给页面作者。页面需要手写 `UiId`、`UiNode`、`UiStyle`、尺寸、颜色和交互标记；命中后又要在另一处把 `UiId` 解释成业务行为。

升级应保留 `punctum-ui` 的纯函数性质。不要引入 QML、CSS、回调闭包、全局节点注册表或 GPU 依赖。

目标 API 是：

```text
页面状态
  -> 纯函数构建 UiTree<Action>
  -> resolve(viewport)
  -> UiFrame<Action>
  -> 输入命中或焦点导航
  -> Action
  -> PresentationState / GameCommand
```

结构 ID 由树自动生成。页面只在有交互意义时声明类型安全的 `Action`。动态集合如图鉴条目只在需要保留焦点、滚动位置或动画身份时声明 `UiKey`。

## 当前问题

### 1. `UiId` 同时承担两种职责

`UiId` 既是树内唯一节点标识，又是 `UiFrame::hit_test` 的返回值。因此每个节点都需要手写 `UiId(n)`，可交互节点还需要依赖这些数字与外层逻辑对齐。

这导致三类重复实现：

| 位置 | 当前做法 | 问题 |
| --- | --- | --- |
| `game-view` | 构树后用私有 `with_generated_ui_ids` 重写所有 ID | 页面仍要先写占位整数，自动化逻辑只在该 crate 存在。 |
| `map-editor-view` | `UiIds` 计数器和固定 action ID 混用 | 结构 ID 与 `EditorIntent` 发生耦合。 |
| `tile-editor` | 另一份 `UiIds` 计数器和 action ID | 与地图编辑器重复，且构树函数需要 `&mut UiIds`。 |

这不是业务需要。大多数静态容器、文字和图片不需要被页面命名。

### 2. 交互是布尔值，不是业务合同

当前 `UiStyle::interactive` 只决定是否生成命中矩形。`hit_test` 返回 `Option<UiId>`。调用方必须知道每个数字代表什么，再映射为 `EditorIntent`、选择项或命令。

这使“画按钮”和“处理按钮”分散在两个模型中。编译器无法检查某个可点击节点是否有合法业务含义。

### 3. 页面在直接使用低层布局语言

图鉴和战斗页面直接构造多层 `UiNode`。每个节点都要重复 `UiStyle` 字段和视觉值。相同的面板、标题、列表项、按钮、精灵框和弹窗没有统一的组件 API 或视觉 token。

结果是页面可读性低，视觉统一性依赖人工记忆，后续改色、改圆角、改间距需要跨页面搜索。

### 4. 输入、焦点、滚动和文本没有统一接口

`punctum-ui` 有命中矩形，但没有激活、取消、焦点移动、键盘列表导航、滚动位置或文本测量合同。游戏目前可处理键盘和已提交文本，但页面作者不能用统一方式声明“这个列表可选择”或“这个按钮激活后产生什么”。

### 5. 旧的实施记录已落后于源码

当前 `game-scene-view` 已让图鉴和战斗输出 `UiFrame`，命令控制台也可作为 Pixel UI 覆盖层。旧实施记录仍将战斗和控制台描述为未完成迁移。升级工作开始前，应以源码和测试为当前事实，不应以该记录判断迁移状态。

## 不可违反的边界

| 边界 | 要求 |
| --- | --- |
| `punctum-ui` | 继续是纯 Rust 数据与算法 crate。不得依赖 Grid、GPU、窗口、输入设备、资产加载或游戏状态。 |
| `punctum-gpu` | 只消费已解析的像素绘制计划。不得理解组件、业务 action 或页面路由。 |
| `game-native-plan` | 继续负责内容 ID、图集资源、文本和 GPU 提交计划的转换。 |
| `game-ui` | 继续拥有导航、焦点、菜单和表现瞬态状态；业务事实仍由 `GameSession` 等应用层拥有。 |
| 页面函数 | 保持 `state -> UiTree<Action>` 的纯函数形式。不得保存闭包、窗口句柄或可变全局状态。 |
| Grid 路径 | 地图和编辑器的 Grid 渲染继续存在。Pixel UI 是并行路径，不是替换地图模型。 |

## 目标模型

### 结构身份与业务身份分离

```text
UiNode<Action>
|- 内部 NodeId：树构造时按稳定先序自动分配
|- 可选 UiKey：页面显式提供，用于动态节点跨重建保持身份
`- 可选 Action：只在节点可激活时存在

UiFrame<Action>
|- 绘制命令
|- 命中区域：NodeId + 可选 Action
`- hit_action(x, y) -> Option<&Action>
```

`NodeId` 不再是页面 API。它只用于错误诊断、调试检查和框架内部索引。

`UiKey` 不是计数器。它表示稳定的页面语义，例如 `PokedexEntry(25)`、`TileMaterial(material_id)` 或 `ConsoleSuggestion(index)`。静态节点不需要 key。

`Action` 由调用方定义。游戏可用 `GameUiAction`，地图编辑器可用 `EditorIntent`，瓦片编辑器可用 `TileEditorAction`。`punctum-ui` 不认识这些类型。

### 页面 API 示例

以下是目标形态，不是本次报告附带的实现：

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
enum PokedexAction {
    Select(DexNumber),
    Close,
}

fn pokedex_page(state: &PokedexPageState) -> UiTree<PokedexAction> {
    ui::screen([
        ui::title("宝可梦图鉴"),
        ui::selectable_list(
            state.entries.iter().map(|entry| {
                ui::list_item(
                    UiKey::from(entry.number()),
                    entry.localized_name(),
                    entry.number() == state.selected,
                    PokedexAction::Select(entry.number()),
                )
            }),
        ),
    ])
}
```

这个函数没有 `&mut UiIds`，没有 `UiId(8_201)`，也不返回回调。相同输入得到相同树和相同的自动 NodeId 分配。

## 升级项与优先级

### P0：泛型 Action 与自动结构 ID

**目的**：先消除手写 ID 和 ID 到业务行为的映射。

**当前状态（2026-07-19）**：`punctum-ui` 已提供 `UiNode::auto`、`UiKey`、`UiNode::with_action` 和 `UiFrame::hit_action`。图鉴已迁移为 `UiTree<PokedexAction>`：静态节点自动分配 ID，列表项使用全国图鉴编号作为 `UiKey`，鼠标命中后由 `PresentationState::handle_pokedex_action` 归约。`game-native-plan` 只读取通用 `UiFrame<Action>` 的绘制命令，不理解 action。旧 `UiNode::new(UiId)`、`UiStyle::interactive` 和 `UiFrame::hit_test` 已标记为 deprecated。战斗、控制台、地图编辑器与瓦片编辑器仍走 legacy 路径。

**改动**：

1. 新增 `UiTree<Action>`、`UiNode<Action>`、`UiFrame<Action>`。
2. 树构造阶段按先序遍历分配私有 `NodeId`。
3. 将 `interactive: bool` 替换为节点可选 `action: Option<Action>`。无 action 的节点不会生成可激活命中区域。
4. 新增 `UiKey`。它是可选稳定身份，不用于每个静态节点。
5. 新增 `UiFrame::hit_action(x, y) -> Option<&Action>`。保留低层命中信息给调试和编辑器使用。
6. 保留现有 `UiId` API 一个迁移周期，标记为 legacy。现有编辑器不需要在第一步改动。

**函数式保证**：树构造、ID 分配、布局和命中均为输入到输出的纯计算。`Action` 是数据，不是闭包。自动 ID 只依赖树的稳定遍历顺序。

**验收标准**：

- 构造包含 1,000 个无 key 节点的树不需要调用方提供 ID。
- 相同输入树生成相同 `NodeId` 与命中结果。
- 同一个 `UiKey` 重复出现返回结构化构建错误。
- 顶层覆盖节点的 `hit_action` 优先于底层节点。
- `punctum-ui` 仍然零外部依赖。

### P1：组件与视觉 token 层

**目的**：让页面表达结构和语义，而不是重复填写低层样式。

**当前状态（2026-07-19）**：`game-ui-kit` 已实现 `GameUiTheme`、`screen`、`panel`、`row`、`column`、`stack`、`text`、`image`、`sprite`、`selectable_list_item`、`button`、`modal` 和 `tab_bar`。图鉴已完整使用组件。战斗页的主菜单、招式列表和招式详情已迁移。战斗宝可梦选择页、控制台和通用 `selectable_list` 仍未迁移。

**位置**：新建 `crates/presentation/game-ui-kit`。它依赖 `punctum-ui`，不依赖 GPU、窗口或 `GameSession`。

**内容**：

- `GameUiTheme`：颜色、边框、圆角、间距、字号和层级。
- 基础组件：`screen`、`panel`、`row`、`column`、`stack`、`text`、`image`、`sprite`。
- 交互组件：`selectable_list_item` 携带 `Action`。`button`、`modal`、`tab_bar` 目前只表达视觉结构；页面业务 action 和焦点路由留给 P7。后续增加 `selectable_list`。
- 页面只能通过组件或少数明确的低层 escape hatch 构树。

`punctum-ui` 保留 `UiContent::Image`、`ImageTinted` 和 `ImageStyled` 等图像原语，不新增游戏组件。`game-ui-kit::image` 负责把内容 ID、尺寸和通用样式构造成节点。`game-ui-kit::sprite` 在此基础上封装染色、像素偏移、圆角和裁剪。两者都不能依赖图集、GPU、资源加载或游戏状态。

**函数式保证**：组件只是 `props + children -> UiNode<Action>` 的普通函数。主题值作为显式参数或只读值传入，不使用全局可变主题。

**验收标准**：

- 图鉴的重复面板、列表项、文本和图像构造已移到组件层。
- 战斗主菜单、招式列表和招式详情已使用主题与组件；旧命中区域在 P7 前保持兼容。
- `image` 覆盖普通图像；`sprite` 覆盖普通、染色和带像素偏移的精灵。
- 已迁移的页面函数不再出现 `UiId(n)` 或节点 ID 偏移。
- 改变 `GameUiTheme` 的面板 token 可影响图鉴全部同类组件；第二个页面迁移后再验证跨页面效果。
- 组件层没有业务规则、窗口或 GPU 类型。

### P3：有限滚动与大列表

**目的**：支持图鉴、背包、商店和任务列表，避免页面手工截取固定几行。

**改动**：

1. 新增 `ScrollState { offset, viewport_extent, content_extent }`，归 `PresentationState` 或页面状态所有。
2. `punctum-ui` 只支持裁剪区域和显式偏移，不在布局 core 中引入惯性或平台事件。
3. `game-ui-kit::selectable_list` 根据页面选择状态和 `ScrollState` 构造可见项；P7 后再接入通用焦点。
4. 首版用窗口化构树：只构造可见项与缓冲区。数据源与 action 由页面提供。

**验收标准**：

- 386 条图鉴记录可键盘滚动，任意时刻构树数量受可见区和缓冲区上限控制。
- 选中项移出可见区时自动滚入视图。
- 裁剪、命中与绘制顺序在滚动后仍一致。

### P4：文本测量与换行

**目的**：从“调用方猜文字大小”升级为可验证的文本布局合同。

**改动**：

1. 定义后端无关的 `UiTextMetrics` 输入和 `UiMeasuredText` 结果。
2. 页面或 native adapter 先测量文字，再将固有尺寸与断行结果提供给 `punctum-ui`。
3. 增加单行截断、有限行数换行和省略号。富文本、选择和编辑态不属于这一阶段。

**验收标准**：

- 中英文混合标题在指定最大宽度内有稳定的换行结果。
- 改变 viewport 时不会出现文字压住相邻组件。
- 布局测试不需要创建窗口或 GPU device。

### P5：语义与可访问性投影

**目的**：让 UI 有可访问名称、角色、状态和 action，而不是只能被绘制。

**改动**：

1. 在组件层引入 `UiSemantics`：角色、名称、描述、选中、禁用和值。
2. 从 `UiTree<Action>` 生成独立 `UiSemanticsTree<Action>`。
3. 原生 adapter 后续可将其接到平台辅助技术；在此之前先用测试验证语义树。

**验收标准**：

- 每个可激活组件必须有名称与角色。
- 选中列表项和禁用按钮具有可观察状态。
- 语义 tree 不依赖 GPU 像素或图像识别。

### P6：渲染验收与开发工具

**目的**：防止框架升级改变页面视觉或隐藏性能问题。

**改动**：

- 固定 viewport 的 `UiFrame` 快照测试。
- `UiTree` / `UiFrame` 调试导出，显示 NodeId、UiKey、bounds、clip、action 和焦点。
- native 截图验收：图鉴、战斗、地图加控制台、窗口 resize。
- 记录实例数、draw call、布局耗时和长列表构树数量。

**验收标准**：

- 升级前后的核心页面满足文字、资源键、层级和相对矩形的视觉合同。
- 能定位一次命中为何落到某个 action。
- 性能结论来自基准数据，不从 API 形状推断。

### P7：页面业务焦点与输入路由

**目的**：在页面组件和业务导航形状稳定后，再统一方向键、确认、取消和 pointer activate。

**改动**：

1. `punctum-ui` 继续只导出 `UiKey` 和 action hit 信息，不定义上下左右的通用移动规则。
2. `game-ui` 为每个页面定义焦点路由。列表按业务顺序移动；按钮、标签和弹窗按页面结构定义进入、离开和回退规则。
3. `PresentationState` 在需要时保存当前焦点的 `UiKey`，不保存 NodeId。
4. 键盘和鼠标最终都映射为页面的 `Action`；`GameSession` 只接收已经明确的 `GameCommand`。

**范围限制**：不实现猜测式空间导航、事件冒泡、pointer capture、拖放或手势。焦点顺序必须由页面业务明确声明。

**验收标准**：

- 图鉴、战斗和弹窗各自定义可预测的焦点移动与确认行为。
- 动态列表重建后，焦点按 `UiKey` 保持在同一条目；条目消失时按页面规则回退。
- 浏览或改变焦点不推进世界时间。

## 实施顺序

| 阶段 | 交付 | 首个迁移对象 | 完成条件 |
| --- | --- | --- | --- |
| 1 | P0 自动 ID、`Action`、`UiKey` | `punctum-ui` fixture、图鉴 | 已完成：新旧 API 并存，图鉴可由键盘或鼠标选择条目。 |
| 2 | P1 `game-ui-kit` 与 token | 图鉴 | 基础组件和图鉴迁移已完成；`button`、`modal`、`tab_bar` 已可用。 |
| 3 | P1 迁移 | 战斗、控制台 | 战斗主菜单、招式列表和招式详情已完成；继续迁移宝可梦选择页和控制台，保留现有页面业务输入。 |
| 4 | P3 滚动 | 图鉴完整列表 | 可见项窗口化，选择与滚动稳定。 |
| 5 | P4 文本 | 图鉴、控制台 | 文本测量、换行和截断可验证。 |
| 6 | P5/P6 | 全部 Pixel 页面 | 语义、截图和性能基线可回归验证。 |
| 7 | P7 焦点与激活 | 图鉴、战斗、弹窗 | 页面业务定义焦点路由，键盘和鼠标产生同一 Action。 |
| 8 | legacy 清理 | 地图/瓦片编辑器 | 编辑器迁移到类型化 action 后删除 `UiId -> Intent` 路由。 |

不要先改编辑器。它们正在使用手写 ID 的命中路径，迁移风险比图鉴高。先用图鉴验证新 API，确认 action、key、焦点和滚动的边界后，再处理编辑器。

## 迁移兼容策略

1. P0 先增加新 API，不删除 `UiId`、`UiNode::new(UiId)` 或 `hit_test`。
2. `UiId` API 标记为 legacy，但不立刻废弃，避免阻断地图和瓦片编辑器的现有工作。
3. `game-view` 的图鉴已迁移到 `UiTree<PokedexAction>`，不再使用手写 ID 或私有 `with_generated_ui_ids`。战斗主菜单和招式详情已改用组件，但整页仍通过 `with_generated_ui_ids` 兼容 legacy 命中区域；控制台仍待迁移。
4. `map-editor-view` 迁移到 `UiTree<EditorIntent>` 后，删除 `intent_for_ui_hit` 的数字映射。
5. `tile-editor` 迁移到 `UiTree<TileEditorAction>` 后，删除本地 `UiIds` 计数器。
6. 三条路径完成且 native 输入验收后，才删除 legacy ID API。

## 明确不做的事

- 不实现 CSS 选择器、样式级联、完整 Flexbox 或浏览器兼容层。
- 不在 UI 树保存闭包、可变状态、窗口句柄或 GPU resource。
- 不让 `punctum-ui` 解析按键、打开窗口、读取资产或产生 `GameCommand`。
- 不用 NodeId 持久化焦点、滚动或业务选择。
- 不在没有真实页面需求时实现虚拟 DOM、惯性滚动、拖放、手势或通用表格。
- 不为升级 UI API 重写地图 Grid、WGPU runtime 或游戏领域模型。

## 风险与控制

| 风险 | 控制 |
| --- | --- |
| 泛型 Action 扩散导致 API 复杂 | 只让 `UiTree`、`UiNode`、`UiFrame` 带 Action。布局几何和 GPU 计划仍不带业务类型。 |
| 自动 ID 因列表重排而变化 | NodeId 只作内部身份；跨重建状态一律使用显式 `UiKey`。 |
| 组件层重新实现业务逻辑 | `game-ui-kit` 只返回节点和 action，不读取或修改 `GameSession`。 |
| 迁移改变战斗视觉 | 战斗主菜单和招式详情已有 frame 测试；继续迁移宝可梦选择页和控制台时必须补 frame/screenshot 对比。 |
| 滚动变成无边界的通用控件工程 | P3 仅支持列表所需的显式 offset、clip 和窗口化。 |
| 新 API 没有真正减少页面代码 | 每阶段测量迁移页面的 `UiId`、重复 style 字段和本地 ID 分配器数量。 |

## 当前事实与验证范围

本报告基于当前源码：

- `punctum-ui` 的泛型树、自动 ID、`UiKey` 与 typed action API。
- `game-view` 的图鉴 `UiTree<PokedexAction>`；战斗和控制台仍有 `with_generated_ui_ids` 本地补丁。
- `game-scene-view` 和 `game-host` 保留图鉴的 typed frame，并分发鼠标命中 action。
- `map-editor-view`、`tile-editor` 的 `UiIds` 与 `UiId -> Intent` 路由。
- 当前 Grid/Pixel 双路径和 `game-scene-view` 的场景组合。

本次验证命令：

```powershell
cargo test -p game-ui-kit -p punctum-ui -p game-ui -p game-view -p game-scene-view -p game-native-plan -p game-host
```

结果：57 个测试通过；`game-host` 的 1 个资产完整性测试按已知资源缺口忽略。旧页面和编辑器使用 deprecated API 会产生预期警告，迁移完成前不将这些警告升级为构建错误。
