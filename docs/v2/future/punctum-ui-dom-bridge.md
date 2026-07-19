# Punctum UI 与 DOM 接入

> 分类：未来；最后整理：2026-07-20。
> 状态：提案。当前 workspace 没有 Wasm、`web-sys` 或 DOM renderer。

## 结论

`punctum-ui` 不能直接接入或生成 DOM 标签。它不是 HTML 模板系统。

它可以作为 Web UI 的纯布局和交互模型。浏览器侧的 adapter 可以消费它的解析结果，再创建 canvas 绘制或 DOM 节点。DOM、CSS、ARIA、焦点和浏览器事件必须留在 Web adapter/runtime，不能进入 `punctum-ui`。

首版 Web runtime 应采用混合结构：游戏场景和现有 Pixel UI 绘制到 canvas；浏览器必须提供的界面使用独立 DOM 岛。不要把同一个页面同时交给 CSS 和 `punctum-ui` 布局。

```text
浏览器页面
  game-web-host
    ├─ canvas：地图、sprite、现有 Pixel UI
    │    UiTree<Action> -> UiFrame<Action> -> game-web-target
    └─ DOM 岛：启动失败、加载状态、设置、文件导入导出、无障碍入口
         浏览器事件 -> 归一化输入/类型化动作 -> presentation/application
```

这样可以复用现有游戏 UI，同时让浏览器专有能力有明确归属。

## 当前代码能提供什么

`punctum-ui` 的 `UiNode` 只包含样式、填充、图片、文字、子节点、稳定 key 和可选的类型化 action。`UiContent` 目前只有 `Empty`、`Fill`、`Image`、`ImageTinted`、`ImageStyled`、`Text` 与 `TextScaled`。它没有标签名、属性、CSS class、ARIA role、焦点状态、表单值或浏览器事件类型。

`UiTree::resolve(UiSize)` 会生成 `UiFrame<Action>`。frame 中是有序的 `UiDrawCommand`、命中区域和 `UiActionHit<Action>`。绘制命令只有填充、图片和文字；它们适合交给 GPU/canvas renderer。现有 native 路径正是由 `FramePlan::from_ui_frame` 消费这些命令。

因此，以下说法需要区分：

| 目标 | 当前是否可做 | 结论 |
| --- | --- | --- |
| 在浏览器 canvas 中显示现有 Pixel UI | 可以 | Web target 消费 `UiFrame`，与 native target 一样保持 `punctum-ui` 无平台依赖。 |
| 在 Web runtime 旁放置原生 DOM 控件 | 可以 | `game-web-host` 直接拥有这些节点和事件监听器。 |
| 把每个绘制命令变成 `div`、`img`、`span` | 可以，但只适合调试或简单覆盖层 | 这只能复现像素外观，不能推导出正确语义。 |
| 把动作节点可靠地变成 `button`、`input`、列表等语义标签 | 不能直接做 | 当前 frame 已丢失节点层级和语义，`Action` 也不能反推出可读标签。 |
| 用 CSS 接管现有 `UiTree` 的布局 | 不应做 | CSS 的文本度量、flex 规则和裁剪会与 `UiTree::resolve` 的确定性像素布局发生漂移。 |

## 推荐 Web 页面结构

`game-web-host` 是页面组合根。它创建并销毁 DOM 节点、canvas、浏览器事件监听器和请求动画帧。`game-web-target` 只负责把场景和 `UiFrame` 提交到 canvas；它不决定页面结构。

页面可保留一个全屏 canvas，并在同级容器放置少量 DOM 岛：

```html
<main id="game-web-host">
  <canvas id="game-surface"></canvas>
  <section id="game-web-overlay" aria-live="polite"></section>
  <input id="game-file-import" type="file" hidden>
</main>
```

`game-web-overlay` 只承载不适合画进游戏画布的内容，例如启动错误、资源加载失败、暂停说明、设置面板、文件选择和辅助技术入口。它不应成为第二套游戏 HUD。

同一时刻，一个视觉区域只能有一个布局权威：

- 游戏 canvas 内的 UI 由 `UiTree::resolve` 的逻辑像素坐标决定。DOM 不重算这些元素的位置。
- DOM 岛由浏览器布局决定。它不再构造相同区域的 `UiTree`。
- DOM 事件先转换为 `punctum-input` 事件或明确的 Web UI action，再进入现有 presentation/application 输入路径。DOM 回调不得直接修改 `GameSession`。

## 不要把 UiFrame 直接当作语义 DOM

可以编写一个试验性的 renderer，把 `UiDrawCommand::Fill`、`Image`、`Text` 转成绝对定位的 `div`、`img`、`span`。它应把 `UiFrame` 的像素坐标作为唯一布局结果，并以 `UiId`/`UiKey` 做 DOM diff 的身份键。

这个 renderer 不能成为通用 Web UI 的承诺，原因如下：

- frame 是扁平的绘制序列，保留绘制顺序，但不保留容器、子节点和语义角色；
- 一个交互节点可能只画出背景和文字，无法从这些命令判断应该是 `button`、`a` 还是可选择列表项；
- `Text` 没有可访问名称、语言、换行策略或输入状态；
- 当前 core 还没有通用焦点、Tab 顺序、滚动、文本测量/换行和可访问性树；
- 浏览器字体度量与现有像素文字路径不同。让 CSS 重新布局会改变命中区域和视觉结果。

试验 renderer 的用途仅限于布局检查、开发工具或固定内容覆盖层。游戏内的像素菜单仍应优先走 canvas。

## 需要语义 DOM 时的扩展方式

当某个页面确实需要键盘焦点、表单输入、屏幕阅读器或浏览器原生控件时，先确认它是否应作为 Web DOM 岛独立实现。只有同一份页面必须同时支持 native Pixel UI 和语义 DOM 时，才扩展 `punctum-ui`。

扩展必须保持两层职责：

| 层 | 应增加的内容 | 不应增加的内容 |
| --- | --- | --- |
| `punctum-ui` | 与平台无关的 `UiSemantics`，例如 role、可访问名称、禁用状态、焦点顺序和稳定节点身份；每个已布局节点的 bounds/clip/层级快照。 | `HtmlElement`、`web_sys`、CSS 字符串、DOM 事件监听器、浏览器存储。 |
| `adapter/punctum-dom` 或 `adapter/game-web-target` | `UiSemantics` 到具体标签、ARIA 属性、绝对定位样式、焦点同步和 DOM 事件的映射。 | 游戏规则、`GameSession` 内部状态、领域 action 的字符串编码。 |
| `runtime/game-web-host` | 页面根节点、canvas 生命周期、DOM 岛装配、文件选择、可见性和浏览器错误展示。 | 可复用布局规则或领域状态转换。 |

为避免把 `UiFrame` 的扁平绘制命令误当作 DOM 模板，core 应增加一个独立的已解析节点输出，例如 `UiResolvedNode`。它至少需要包含 `UiId`、可选 `UiKey`、bounds、clip、层级关系、语义和动作是否存在。具体 DOM 标签仍由 adapter 选择。

`UiSemantics` 必须是可选的。没有语义的游戏像素节点仍可走 canvas，不应被迫伪装成按钮或输入框。`Action` 继续是类型参数；DOM adapter 只把其对应节点的浏览器事件送回运行时，不把 action 序列化为 DOM 属性。

## 实施顺序

1. 按 [Web runtime 与 WebGL 降级方案](web-runtime-webgl.md) 先完成 canvas、资源加载和最小游戏帧的 Wasm 冒烟验证。
2. 在 `game-web-host` 加入静态 DOM 岛，只处理启动错误、加载状态和一个文件导入或设置流程。验证 DOM 事件会被归一化，而不会绕过状态所有者。
3. 为 DOM 岛建立焦点、页面隐藏、缩放和 canvas 指针坐标换算的浏览器测试。游戏 canvas 和 DOM 覆盖层的点击优先级必须明确。
4. 只有出现跨 native/Web 的语义页面需求时，设计 `UiSemantics` 与已解析节点输出，并先在纯 Rust 测试中固定其身份、布局和失败合同。
5. 新增 DOM adapter。它以已解析节点输出构建和更新 DOM；浏览器测试验证 role、可访问名称、焦点顺序、事件映射和缩放后命中位置。

## 验收条件

- `punctum-ui`、domain、application 和 presentation 不依赖 `web-sys`、Wasm 条件编译或 DOM 类型。
- 现有 `UiFrame` 能在 Web canvas 路径绘制，且命中测试继续以同一逻辑坐标系工作。
- DOM 岛不会重复渲染 canvas 中的游戏 HUD，也不会直接修改 `GameSession`。
- 需要语义的控件具有可验证的 role、可访问名称、键盘焦点和禁用状态。
- DOM 的缩放、设备像素比、canvas 尺寸变化和覆盖层点击顺序都有浏览器验收。
- 原生 `game-host` 的 `UiTree -> UiFrame -> FramePlan` 路径和测试保持不变。

## 代码依据

- `crates/foundation/punctum-ui/src/model.rs`：当前内容和样式模型不含 DOM 或无障碍语义。
- `crates/foundation/punctum-ui/src/tree.rs` 与 `layout.rs`：tree 解析为绘制命令、命中区域和类型化 action。
- `crates/presentation/game-native-plan/src/lib.rs`：native 计划只消费 `UiDrawCommand`，不理解页面语义。
- `docs/v2/current/007-UI/006-UI树到帧计划.md`：当前 Pixel UI 的纯编译和 native 提交边界。
