# 图鉴筛选 Form 与双场景重构

> 分类：未来；状态：已定设计，待实现。

## 结论

图鉴保留两个主场景：`Browse` 和 `Detail`。

- `Browse` 是三项竖向拨轮，用来选择宝可梦。
- `Detail` 固定保留一份左侧精灵结果列表。右侧在资料和技能之间切换。
- 精灵筛选和技能筛选使用同一个键盘 Form 框架。它不是第三张图鉴页面。
- Form 有紧凑态和全屏态。两种状态使用同一份筛选条件和焦点模型。
- `Ctrl+F` 与点击放大镜是同一个动作：切换全屏 Form 与紧凑态。

当前图鉴的 `Browse -> Profile -> Moves` 三段横向场景会删除。条形种族值图和它的切换动作也会删除。

## 目标与边界

本轮目标：

- 键盘优先地筛选精灵和当前精灵的可学技能。
- 不挤压正常详情画面。筛选条件收起时只显示为小型状态摘要。
- 让拨轮、详情左侧列表和技能列表使用同一份筛选结果。
- 将 Form 焦点、输入和控件行为沉到可复用组件，避免图鉴实现一套、背包再实现一套。
- 详情页显示真实数据：名称、中文分类、属性、身高、体重、特性、雷达图和六项种族值。

本轮不做：

- 不接入图鉴描述句子。当前数据源没有本地化图鉴文本。
- 不做传说、幻之、究极异兽、最终进化筛选。当前导入数据没有可靠的物种分类和进化链。
- 不做技能描述关键词搜索。本轮搜索范围仅为技能名称。
- 不引入运行时 SVG 渲染。筛选入口使用普通 Sans 可渲染的 UTF-8 字符 `⌕`，不新增图标资源。
- 不改世界画面的 HUD，也不加返回按钮。

## 视图结构

### 主场景

```text
                         Right / Confirm
Browse  ------------------------------------------------->  Detail
三项竖向拨轮                                                   左侧结果列表 + 右侧内容
中心：320 icon、名称、编号                                     资料 <-> 技能
邻项：160 icon
                         Left
```

`Browse -> Detail` 保留已有的共享 icon 转场：当前三项从 `320 / 160 / 160` 缩放并移动到详情左侧列表的对应行。转场的起点和终点必须基于实际 viewport 布局计算。

`Detail` 内部不再创建横向新页面。资料与技能只替换右侧内容，左侧精灵列表不移动。右侧使用短位移加淡入，目标时长 `180ms`；新输入从当前视觉位置继续，不回跳。

```text
紧凑态 Detail

+------------+----------------------------------------------+
| [⌕]         |  NO.001  妙蛙种子                             |
| 001  icon   |  [草] [毒]                                   |
| 002  icon   |                                              |
|>003  icon   |             六维雷达图                       |
| 004  icon   |                                              |
| 005  icon   |  分类 / 身高 / 体重 / 特性                  |
+------------+----------------------------------------------+

Detail 的技能模式

+------------+----------------------------------------------+
| [⌕]         |  [类别图] 01 技能名                          |
| 001  icon   |  [类别图] 02 技能名                          |
|>002  icon   |  [类别图] 03 技能名                          |
| 003  icon   |                                              |
| 004  icon   |  当前技能：属性、威力、命中、PP、习得方式    |
+------------+----------------------------------------------+
```

查询入口使用 UTF-8 字符 `⌕` 作为图标按钮。它同时表达“当前列表可筛选”和“已有筛选条件”。存在有效条件时，图标旁只显示属性图标、类别图标或短数值范围摘要，不显示长说明文字；不显示裸结果数字。

展开后的 Form 不覆盖为一整块实色页面。它以 `modal_scrim` 暗化底下的图鉴场景，再在画面中心放置带圆角、边框和裁剪的 `card` 表面。遮罩和边框都是 `GameUiTheme` token：页面不得自行写颜色。Form 的滚动只更新这个卡片内部，不触发图鉴场景切换。

### 全屏 Form

`Ctrl+F` 或点击放大镜会用全屏 Form 覆盖图鉴内容。覆盖层使用图鉴主题的实色背景，不使用黄色蒙层。关闭后回到紧凑态和此前的列表焦点。

```text
+--------------------------------------------------------------+
| [重置]                                                        |
|                                                              |
| 属性     [火] [水] [草] [电] ...    匹配 (任一) / (全部)     |
| 世代     [关都 v] [城都 v] [丰缘 v]                          |
| 身高     最小 [      ]  ~  最大 [      ]                     |
| 体重     最小 [      ]  ~  最大 [      ]                     |
| 特性     [输入名称或选择特性                         v]      |
|                                                              |
|                    筛选结果在底层列表实时更新                |
+--------------------------------------------------------------+
```

技能 Form 使用同一骨架，只替换字段：

```text
+--------------------------------------------------------------+
| [重置]                                                        |
|                                                              |
| 技能名称 [                                      x ]          |
| 属性     [火] [水] [草] [电] ...                             |
| 分类     (全部) (物理) (特殊) (变化)                         |
| 威力     最小 [      ]  ~  最大 [      ]                     |
| 命中     [所有命中                                      v]    |
| 开关     [○ 仅看有先制度]                                   |
+--------------------------------------------------------------+
```

## 键盘合同

### 主场景

| 位置 | 上/下/Home/End | 左 | 右 | Confirm | Ctrl+F | Cancel |
| --- | --- | --- | --- | --- | --- | --- |
| Browse | 切精灵 | 无 | 进 Detail 资料 | 进 Detail 资料 | 打开精灵 Form | 关闭图鉴 |
| Detail 资料 | 切左侧精灵 | 回 Browse | 进技能模式 | 无 | 打开精灵 Form | 关闭图鉴 |
| Detail 技能 | 切技能 | 回资料模式 | 无 | 保持当前技能 | 打开技能 Form | 关闭图鉴 |

点击左侧精灵行后，后续上下键必须从被点击的筛选结果索引继续。点击技能也遵守同一规则。

### Form

- `Ctrl+F` 和放大镜点击使用同一个 `ToggleFilterForm` 语义动作。
- Form 紧凑时，动作进入全屏态；全屏时，动作收回紧凑态。
- `Tab` 和 `Shift+Tab` 只在全屏 Form 内按固定顺序移动字段焦点。图鉴外的 `Tab` 行为不改。
- 文本输入只消费 `TextEvent`。不能从 `LogicalKey::Character` 拼接文本。
- 标签组：方向键移动组内焦点，`Space` 切换多选。
- 单选组：方向键移动组内焦点，`Space` 或 `Enter` 选中。
- 数值范围：输入暂停 `300ms` 后提交有效数值；`Esc` 清空当前输入框；最小值大于最大值时保留草稿但不更新结果。
- 可搜索下拉：`Enter` 打开，输入过滤，方向键选择，`Enter` 提交，`Esc` 先清搜索词，再关闭下拉。
- 开关：`Space` 或 `Enter` 立即切换。
- `Esc` 的优先级为：清当前文本/关闭下拉 -> 收起全屏 Form -> 原有关闭页面行为。

精灵 Form 的 Tab 顺序：属性组、属性匹配模式、世代组、身高最小、身高最大、体重最小、体重最大、特性下拉、重置。

技能 Form 的 Tab 顺序：名称、属性组、分类组、威力最小、威力最大、命中下拉、优先度开关、重置。

## 状态与组件模型

### 分层

```text
punctum-ui
  KeyboardFormState<ItemId>
  KeyboardForm<ItemId> / FormItem<ItemId>
          |
game-ui-kit
  KeyboardForm 的像素 UI 投影
  文本、多选、单选、范围、下拉、开关、重置
          |
game-ui
  PokedexFilterModel / MoveFilterModel
  PokedexFilterForm / MoveFilterForm 的字段定义
  键盘、TextEvent、动画和列表焦点
          |
game-view
  只读取 PageModel 与 PokedexVisualState
  投影筛选后的拨轮、左列表、资料或技能
```

Model 和 Form 必须分离。

`KeyboardFormState<ItemId>` 是通用交互状态。它不保存筛选值、字段文本或页面业务数据。它只保存：

- `presentation`：`Compact` 或 `Expanded`。
- `focused_item`：当前 FormItem 的稳定 ID。
- `opened_select`：当前展开的下拉项。

`KeyboardForm<ItemId>` 是无状态字段定义。它保存固定 Tab 顺序和每个 `FormItem<ItemId>` 的种类、稳定 ID、可见性和可用性。它不保存当前选中值，也不创建页面业务动作。

`game-ui-kit` 根据 `KeyboardForm`、`KeyboardFormState` 和页面提供的字段值构建像素 UI。它不修改筛选条件。

`PokedexFilterModel` 和 `MoveFilterModel` 是强类型业务模型。它们保存已生效条件、文本和数值编辑草稿、字段校验结果与 `300ms` 防抖时钟。它们不能退化为 `HashMap<String, String>`。

```text
PokedexFilterModel
  type_ids: Set<TypeId>
  type_match: Any | All
  generations: Set<Generation>
  height: Range<Option<u16>>
  weight: Range<Option<u16>>
  ability: Option<AbilityId>

MoveFilterModel
  name_query: String
  type_ids: Set<TypeId>
  category: Option<PokedexMoveCategory>
  power: Range<Option<u16>>
  accuracy: Option<MoveAccuracy>
  priority_only: bool
```

每种 FormItem 都将输入转换为语义变更，例如 `SetHeightMin`、`ToggleType`、`SetMoveCategory`。`game-ui` 处理这些语义变更并重新计算结果索引。`game-view` 不保存筛选状态，也不直接处理按键。

```text
Ctrl+F / 放大镜
        |
KeyboardFormState 切换 Compact <-> Expanded
        |
KeyboardForm 按稳定 ItemId 路由 Tab、方向键和 Esc
        |
PokedexFilterModel / MoveFilterModel 接收语义变更
        |
筛选结果、摘要和列表重新投影
```

## Form 组件集

`FormItem` 是字段协议，不是视觉控件。`game-ui-kit` 需要提供下列可组合组件。页面 Form 只组合组件，不复制它们的焦点和输入实现。

| 组件 | 用途 | 键盘合同 |
| --- | --- | --- |
| `FormShell` | 全屏 Form 的背景、裁剪、内部滚动和统一内边距。 | 全屏态接管 Tab 焦点；紧凑态不参与 Tab。 |
| `FormSection` | 组织一组相关字段，例如“属性”或“体型”。 | 只负责布局，不单独获得焦点。 |
| `Label` | 显示字段名称、单位、错误或简短辅助信息。 | 不可聚焦。 |
| `IconButton` | 放大镜、清空和重置等纯命令入口。 | Tab 聚焦后以 `Enter` 或 `Space` 触发；放大镜的键盘等价物是 `Ctrl+F`。 |
| `TextInput` | 技能名称和下拉搜索词输入。 | 只写入 `TextEvent`；`Backspace` 删除；`Esc` 清空当前文本。 |
| `NumberInput` | 身高、体重和威力的单个数值边界。 | 只接受合法数字和一个小数点；失焦或停输 `300ms` 后请求 Model 校验。 |
| `RangeInput` | 成对组合最小/最大 `NumberInput`，并显示区间状态。 | Tab 依次进入最小、最大；无效区间不提交。 |
| `CheckboxGroup` | 多选属性和多选世代。 | 方向键在组内移动，`Space` 选中或取消；每项独立。 |
| `RadioGroup` | 属性匹配模式和技能分类。 | 方向键在组内移动，`Space` 或 `Enter` 选中唯一项；“全部”是显式选项。 |
| `Select` | 特性和命中率的单选下拉。 | `Enter` 展开；文本过滤或方向键移动；`Enter` 提交；`Esc` 逐级关闭。 |
| `Toggle` | 仅看有先制度等独立二元条件。 | `Space` 或 `Enter` 切换。 |
| `FilterSummary` | 紧凑态展示查询图标和生效条件摘要。 | 只让查询和重置命令可操作，摘要项本身不可聚焦。 |

组件的值都由页面 Model 传入。组件只能发出语义事件，不得直接修改 `PokedexFilterModel` 或 `MoveFilterModel`。

```text
PokedexFilterForm
  FormShell
    FormSection
      Label + CheckboxGroup + RadioGroup
      Label + RangeInput
      Label + Select
    IconButton(reset)

MoveFilterForm
  FormShell
    Label + TextInput
    Label + CheckboxGroup
    Label + RadioGroup
    Label + RangeInput
    Label + Select
    Toggle
```

## 筛选语义

- 不同维度之间使用 AND。
- 同一精灵属性组按 `Any` 或 `All` 模式匹配。
- 技能属性多选按任一属性匹配。每个技能只有一个属性。
- 世代由全国图鉴号稳定推导：关都 `1..=151`、城都 `152..=251`、丰缘 `252..=386`。
- 数值范围是闭区间。空的一端表示不设限。
- 命中下拉包含“所有命中”和“必定命中”。`accuracy=None` 代表必定命中，不能伪装为 `100%`。
- 技能名搜索使用大小写无关的本地化名称匹配。
- 筛选结果为空时，列表保留稳定尺寸并显示空态；没有可选中项目。
- 当前选择仍在新结果中时保持选择；否则选择结果第一项。技能选择变化后保留该精灵，精灵变化后将技能选择重置到结果第一项。
- 真实产品模式下，未知精灵不参与依赖属性、身高、体重、特性或种族值的筛选。这样不会借筛选条件泄露未发现信息。

## 数据链路

当前 `gen3.v1` 图鉴包只包含名称、属性、种族值和 front 资源，不能满足详情和筛选。

本轮将生成新版本图鉴数据包，并更新加载器与投影：

| 字段 | 现有来源 | 用途 |
| --- | --- | --- |
| 中文分类 | `pokemon_species_names.csv` 的中文 `genus` | 详情显示 |
| 身高、体重 | `pokemon.csv` | 详情和范围筛选 |
| 特性、隐藏特性 | `pokemon_abilities.csv`、`ability_names.csv` | 详情和特性筛选 |
| 技能优先度 | `moves.csv` | 技能优先度开关 |
| 技能属性、分类、威力、命中 | 现有 `moves.csv` 投影 | 技能筛选 |

`PokedexEntryModel` 只向已发现条目暴露这些事实。页面模型仍负责将 `PokedexData` 和 `CurrentDataSet` 变成可显示的页面事实；UI 层只保存筛选条件与焦点。

## Crate 改动

| Crate | 改动 |
| --- | --- |
| `punctum-ui` | 分别新增无状态 `KeyboardForm`、`FormItem` 与纯交互 `KeyboardFormState`。负责稳定 FormItem ID、焦点遍历、展开状态和下拉状态；不保存字段值或图鉴业务。保留现有虚拟滚动组件。 |
| `game-ui-kit` | 新增 `FormShell`、`FormSection`、`Label`、`IconButton`、`TextInput`、`NumberInput`、`RangeInput`、`CheckboxGroup`、`RadioGroup`、`Select`、`Toggle` 和 `FilterSummary` 的像素投影。组件只读取值并发出语义事件，不修改 Model。 |
| `game-ui` | 新增强类型精灵/技能筛选 Model，持有已生效条件、编辑草稿、校验和防抖；定义各自的 `KeyboardForm` 字段；将图鉴场景收为 `Browse`、`Detail`；新增详情内部资料/技能模式；处理 `Ctrl+F`、`TextEvent`、筛选结果和焦点恢复。 |
| `game-page-model` | 删除 `PokedexStatsView`、`TogglePokedexStatsView` 及其路由字段；扩展精灵和技能页面事实，投影分类、体型、特性和优先度。`PokedexSection` 从该 crate 移出。 |
| `game-view` | 将 `profile.rs` 和 `moves.rs` 收敛为 Detail 的右侧内容；左侧列表只投影一次且支持筛选结果；资料固定使用雷达图；投影全屏 Form、紧凑摘要和 UTF-8 放大镜入口。 |
| `game-data` | 升级嵌入图鉴数据包格式与加载校验，暴露分类、体型和所需特性关联。 |
| `game-data-import-core` | 从现有 CSV 导入上述字段，保持中文 locale、引用校验和二进制数据生成的一致性。 |
| `game-page-demo` | 复用现有输入事件路径，验证 `Ctrl+F`、全屏 Form、文本筛选和不同 viewport 的布局。 |

## 验收

- `Browse` 和 `Detail` 是仅有的图鉴主场景。不存在独立 Moves 全屏场景。
- 资料和技能模式共用同一份左侧精灵列表。没有重复的精灵列表节点或重复的大精灵图。
- 资料页只显示雷达图，并保留 `50 / 100 / 150` 三层六边形网格与超模值外溢。
- `Ctrl+F` 只在图鉴中展开或收起 Form；鼠标点击放大镜得到相同行为。
- Tab 顺序、文本输入、下拉、范围校验、Esc 优先级均有单元测试。
- 精灵与技能筛选在 `960x720`、窄窗口和高窗口下不重叠；全屏 Form 允许自身滚动。
- 过滤、点击和键盘切换后，选中索引与虚拟滚动窗口一致。
- 未发现精灵不会被属性、体型、特性或种族值条件反推出信息。
- 数据包加载、页面模型、UI 状态和视图投影的相关测试通过；`game-page-demo` 可手动验证完整流程。
