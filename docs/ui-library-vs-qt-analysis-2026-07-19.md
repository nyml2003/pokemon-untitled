# 当前 UI 库与 Qt 对比分析

> 日期：2026-07-19  
> 范围：当前工作区的 `punctum-ui`、`punctum-gpu`、`game-*-view/plan` 与 Qt 6.11 的 Qt Quick、Qt Quick Controls、Qt Widgets、Graphics View。  
> 结论依据：当前源码和 51 个相关 Rust 测试；Qt 侧以官方文档为准。

## 结论

`punctum-ui` 不应被当作 Qt 的同类替代品。它是为本项目像素风游戏界面定制的轻量布局和绘制计划库。它在确定性、业务边界、图集资源和 Grid/Pixel 混合渲染上更贴合当前游戏。

Qt，准确说是 Qt Quick 加 Qt Quick Controls，仍然是功能完整度远高于当前库的通用 UI 平台。若目标是桌面工具、表单、复杂文本、无障碍、多平台发布或设计师直接制作界面，Qt 的投入和成熟度明显更好。

因此不建议用 Qt 重写游戏运行时，也不建议把 `punctum-ui` 扩展成 Qt 的重写版本。应保留当前双路径：地图和编辑器使用 Grid，游戏页面和覆盖层使用 Pixel UI。后续只补齐已被具体页面需求证明的能力。

## 当前库的真实边界

当前 UI 路径如下：

```text
游戏快照 + PresentationState
  -> game-scene-view
  -> UiTree.resolve(viewport)
  -> UiFrame
  -> game-native-plan::FramePlan::from_ui_frame
  -> punctum-gpu::plan_pixels
  -> game-native-target / punctum-wgpu
```

地图保留另一条 Grid 路径。`SceneFrame` 已支持 `Grid`、`Ui`、`GridWithUi` 和 `UiWithUi`，运行时按 pass 顺序合成。这不是“全项目迁到 Flex”。这是两种内容模型共存。

`punctum-ui` 当前提供：

- 像素尺寸和矩形，且没有 `punctum-grid`、GPU、窗口或输入依赖。
- 行、列、叠放三种容器；固定、比例、填充和内容尺寸；最小/最大尺寸、内外边距、间距、主轴和交叉轴对齐。
- 绝对定位、逻辑画布缩放、矩形裁剪、边框、每角圆角。
- 填充、图像、着色图像和文本的后端无关绘制命令。
- 按绘制顺序的最上层命中测试，以及重复 ID、零比例基数和空间不足等结构化错误。

它已经被图鉴、战斗和命令控制台使用。图鉴和战斗是专注页面；命令控制台可覆盖 Grid 世界场景。此前提案中“战斗和命令控制台未迁移”的说法已经落后于当前源码。

`punctum-gpu` 还保留 Grid `Surface` 提交，同时提供不分配 Grid Surface 的 `GpuPixelImage` 与 `plan_pixels`。`game-native-plan` 是 UI 内容 ID、圆角遮罩、像素矩形、GPU 资源和文本标签之间的唯一转换边界。

## 对比基准

本文将 Qt 分为三条路径，不把它们混为一个库：

- **Qt Quick**：QML 的视觉画布、输入、模型视图和动画基础库。
- **Qt Quick Controls**：按钮、输入框、滚动、导航、菜单、对话框等通用控件。
- **Qt Widgets / Graphics View**：传统桌面控件，以及面向大量可交互图元的 2D 场景框架。

对于本项目，最接近 `punctum-ui` 的是 Qt Quick，而不是 Widgets。地图编辑器在缩放、选区和大量图元方面可类比 Graphics View，但当前实现是自己的 Grid/atlas 渲染路径。

## 能力矩阵

| 维度 | 当前 `punctum-ui` 路径 | Qt Quick / Controls | 判断 |
| --- | --- | --- | --- |
| 定位 | 游戏内像素 UI。目标是少量、固定风格的页面与覆盖层。 | 通用应用 UI 平台。QML、控件、模型视图和动画是一体化体系。 | 不在同一产品层级。 |
| 布局 | 受限 Flex：行列、叠放、固定/比例/填充、最小最大、间距、对齐、绝对定位。整数像素结果。 | `RowLayout`、`ColumnLayout`、`GridLayout` 等有最小、首选、最大尺寸、伸缩因子和更完整的布局约束。 | Qt 能力更宽；当前库范围是刻意收窄。 |
| 绘制 | 图集图片、颜色矩形、圆角遮罩、文本标签。绘制命令可纯 Rust 测试。 | 场景图、控件样式、形状、特效、shader 与动画工具链。 | Qt 更丰富；当前库更容易保持像素风与图集一致。 |
| 输入与交互 | `UiFrame` 可做矩形命中；游戏目前主要由 `game-ui` 解释归一化键盘与已提交文本。 | 鼠标、触摸、键盘、焦点、手势、控件默认行为和输入控件是框架能力。 | 当前库缺少通用 pointer/focus/widget 行为。 |
| 文本 | 文本内容、颜色、字号和裁剪矩形会被传给 native 标签路径。没有文字测量服务、自动换行、选择、富文本或编辑控件。 | 有 `TextField`、`TextArea` 等控件，Widgets 还有成熟的富文本与文本编辑支持。 | Qt 显著领先。 |
| 列表、表格、滚动 | 没有滚动容器、虚拟列表、model/delegate 或复用策略。图鉴列表当前直接生成有限条目。 | 有 `ScrollView`、`ListView`、`GridView`、`TableView` 和 delegate/model 体系。 | 数据量增长时是当前库的首要缺口。 |
| 动画 | 动画状态属于 `PresentationState`；页面树和布局库没有通用 transition/animation API。 | 属性动画、状态和 transition 是 Qt Quick 一等能力。 | 游戏特效应继续由游戏表现状态驱动，不必照搬 Qt。 |
| 无障碍 | 没有语义角色、可访问名称、辅助技术 action 或焦点遍历合同。 | `Accessible` 可表达名称、角色、状态、关系和可调用 action；标准控件已有键盘可访问行为。 | 当前库若面向键盘以外的可访问性，必须单独设计语义投影。 |
| 渲染后端 | WGPU + 自有 atlas。Grid 与 Pixel UI 可按 frame pass 合成。 | Qt Quick 场景图通过 Qt 渲染抽象适配多图形 API；Widgets/Graphics View 另有路径。 | 当前路径适合现有资产；Qt 的后端和平台覆盖更广。 |
| 平台与工具 | 当前 runtime 直接依赖 `winit` 和 `wgpu`；无可视化 UI 编辑器。 | 覆盖桌面、移动和嵌入式平台，配套 Qt Creator、Designer/Design Studio 与部署工具。 | 若需要产品化跨平台工具，Qt 优势很大。 |
| 测试与确定性 | 布局、命中和帧计划是纯数据，可不启动窗口/GPU 测试。 | QML 有测试模块，但声明式绑定、事件循环和场景图会带来更多运行时维度。 | 当前库在核心游戏规则的可重复测试上更简单。 |
| 许可 | 工作区许可证是 MIT。 | Qt 采用商业、LGPLv3/GPL 等多种许可，模块可用性和发布方式必须逐项审查。 | 引入 Qt 前必须做发行合规评估。 |

Qt Quick 的官方定义本身就包含视觉画布、输入、模型视图和延迟实例化；Controls 提供按钮、容器、输入、菜单、导航和弹窗。当前库没有试图覆盖这些范围。[Qt Quick](https://doc.qt.io/qt-6/qtquick-index.html) [Qt Quick Controls](https://doc.qt.io/qt-6/qtquickcontrols-index.html)

## 当前库相对 Qt 的优势

### 1. 与游戏状态边界一致

布局 crate 不知道战斗、地图、资源文件、窗口或 GPU。`PresentationState` 只保存导航、焦点和表现时间，业务变更仍通过 `GameCommand` 回到 `GameSession`。Qt 若直接接管 UI，仍可实现同样边界，但需要额外约束 QML 和 C++/Rust 桥接层，默认不会自然得到。

### 2. 与像素资产和地图渲染一致

当前游戏已经有图集、语义 asset key、Grid 地图和 WGPU runtime。Pixel UI 只增加像素提交，不迫使地图、编辑器或资源管线迁移到另一套 scene graph。地图下、UI 上的 pass 顺序也在 `FramePlan::compose` 中显式表达。

### 3. 成本和行为可控

当前实现没有动态语言、控件主题、隐式绑定或多套平台外观。页面成本是明确的 `UiTree -> UiFrame -> FramePlan` 数据变换。对固定画风、有限页面和强游戏状态机，这是优点，不是功能不足的掩饰。

### 4. 纯逻辑测试直接

本次验证运行了 `cargo test -p punctum-ui -p punctum-gpu -p game-native-plan -p game-view -p game-scene-view`，51 个测试全部通过。覆盖布局、裁剪、命中、像素实例、圆角、场景投影和合成计划。这里的结论仅说明逻辑合同通过，不等于真实窗口的视觉、输入和性能已经验收。

## 当前库相对 Qt 的明确缺口

### 1. 没有控件与交互系统

`interactive: bool` 和矩形命中不是按钮、输入框、选择框或菜单。当前缺少 pointer capture、hover/pressed 状态、焦点顺序、Tab 导航、快捷键作用域、拖放和触摸手势。Qt Quick Controls 已覆盖大量这类控件。

### 2. 没有可扩展的数据视图

当前没有滚动状态、惯性、滚动条、虚拟化、可回收 delegate 或表格。Qt 的 `ScrollView` 可以处理可滚动内容，Qt Quick 的模型/视图/delegate 模式也用于列表和表格。若图鉴、背包、商店或任务列表变成大数据集，当前直接建整棵树的方式会先遇到问题。[ScrollView](https://doc.qt.io/qt-6/qml-qtquick-controls-scrollview.html) [Qt Quick 模型视图](https://doc.qt.io/qt-6/qtquick-modelviewsdata-modelview.html)

### 3. 文本能力很薄

文字布局依赖调用方给定的矩形和字号。没有字体 fallback、字形测量、换行、段落、选择、输入法编辑态、富文本或本地化排版策略。当前命令控制台只处理已提交文本，适合简单命令输入，不构成通用文本编辑器。

### 4. 无障碍尚未建立合同

当前 UI 树没有角色、可访问名称、描述、值、可调用 action 或辅助技术事件。Qt Quick 的 `Accessible` 可暴露这些语义，标准控件还支持键盘访问。这个差距不能靠从像素画面反推；必须从页面数据另行投影语义树。[Qt Quick 无障碍](https://doc.qt.io/qt-6/accessible-qtquick.html)

### 5. 视觉与性能结论尚缺实测

虽然有 WGPU 运行时和稳定的实例计划，当前没有看到同一设备、同一页面下的帧时间、内存、draw call、窗口缩放截图或长列表基准。因此不能声称它比 Qt 更快或更省内存。现阶段只能说它的路径更短、职责更窄。

## 不建议的方向

### 不要把 `punctum-ui` 做成 QML/CSS

不要为了“追平 Qt”引入选择器、样式级联、隐式属性绑定、完整 flex 规范、通用脚本运行时或动态对象树。这会破坏当前纯数据、可测试、可预测的边界，也不能在短期追上 Qt。

### 不要将 Qt 嵌入现有游戏主 UI

把 Qt Quick 嵌入 WGPU/Winit 游戏窗口会引入窗口、渲染设备、输入循环、资源格式和语言运行时的双重所有权问题。它不能消除业务 UI 设计工作，只会把风险从布局代码转移到集成层。除非项目决定改为 Qt 主程序和 QML 主 UI，否则不应做半嵌入式迁移。

### 不要用 Qt Widgets 替换地图或战斗画面

Widgets 适合传统桌面应用。它不是当前像素游戏场景和图集渲染的自然替代。Graphics View 也能管理缩放与大量 2D item，但迁移会绕开现有 Grid/atlas 资产模型，而不是解决当前页面架构问题。

## 建议的路线

### 游戏运行时：继续投资当前库

按以下优先级补齐能力。每一项都应由真实页面驱动，并提供纯逻辑测试和 native 验收。

1. 为 `UiFrame` 增加输入路由合同：pointer down/up/move、焦点、激活和取消。输入映射仍由 `game-ui` 所有。
2. 实现有限滚动容器和键盘列表导航。先满足图鉴、背包和商店；暂不实现通用虚拟 DOM。
3. 建立文本测量接口与换行策略。测量实现可留在 native adapter，布局核心只接收明确的固有尺寸。
4. 增加语义投影：从 `UiTree` 或页面 view model 生成可访问名称、角色、状态和 action。不要由 GPU 像素反推。
5. 补 native 验收和基准：固定分辨率与 resize 截图、地图加控制台合成、输入焦点、draw call/帧时间、长列表内存上限。

### 工具型产品：允许单独评估 Qt

若未来需要完整的地图制作工具、资产管理器、数据表格、表单、文本编辑器、可访问桌面应用或移动端工具，应独立评估 Qt Quick 或 Widgets。这个选择应是新工具的运行时决策，不应反向渗入游戏 core 和 `punctum-ui`。

评估前需要先确认三件事：目标平台、是否需要设计师直接编辑 UI、发行许可是否能满足 Qt 模块的商业/LGPL/GPL 条件。Qt 官方明确指出不同模块的开源许可可用性不同，不能只看框架总许可证。[Qt Licensing](https://doc.qt.io/qt-6/licensing.html)

## 验证范围与证据

本报告核查了下列当前源码：

- `crates/foundation/punctum-ui/src/lib.rs`：纯 UI 树、Flex 子集、绘制命令、裁剪、命中和错误模型。
- `crates/foundation/punctum-gpu/src/plan.rs`：Grid 与像素提交计划。
- `crates/presentation/game-native-plan/src/lib.rs`：`UiFrame` 到 GPU/text 计划的转换与 frame 合成。
- `crates/presentation/game-scene-view/src/lib.rs`：世界、战斗、图鉴和控制台的 Grid/Pixel 场景选择。
- `crates/runtime/game-host/src/main.rs`：各类 `SceneFrame` 到 `FramePlan` 的实际运行时分派。

已运行并通过：

```powershell
cargo test -p punctum-ui -p punctum-gpu -p game-native-plan -p game-view -p game-scene-view
```

未执行：真实 GPU 窗口截图验收、性能基准、辅助技术验收、Qt 原型或迁移。这些未执行项不影响架构对比结论，但限制了视觉和性能判断。
