# 设计

## 状态边界

`PageDemoContext` 提供静态内容和只读产品快照，`PageState` 只保存当前页面路由与局部选择，`game-ui::PageUiState` 保存页面焦点。键盘先由 `PageUiState` 转换为语义 intent；鼠标悬停只改变焦点，点击命中已 resolve 的 `UiFrame<PageIntent>` 后，runtime 将 intent 交给 `PageState::transition`，再从同一快照重建 `PageModel`。

runtime 不执行 reducer 返回的 `PageEffect`。购买和存档请求只转为短反馈文字，因此 demo 不会修改产品状态、文件或地图世界。

## 呈现边界

壳注册 `solid/white` 和当前已确认的页面图片资源，并通过 `FramePlan::from_ui_frame` 将 `game-view::project_page_model_with_notice` 的输出交给 `NativeTarget`。页面投影只产生语义资源键，不携带文件路径；demo 当前绑定角色、固定的树 tile、Treecko 队伍立绘和 Treecko 图鉴立绘。它不执行地图逻辑，只使用固定 tile 验证世界画面的构图。没有对应素材的物种仍显示几何槽位，避免页面翻页时引用不存在的资源。

## 输入与选择

鼠标点击使用 `UiFrame::hit_action` 获取最上层的类型化 action，键盘焦点使用稳定的 `UiKey` 找到对应 action。窗口、坐标、鼠标事件和物理按键都不进入 `game-page-model`。

默认键位是方向键或 WASD 移动焦点，Enter 或 Z 确认，Escape 或 X 取消，Tab 从世界画面打开导航，F5 打开保存确认，Q/E 切换背包分类。页面不渲染返回按钮；取消由键盘状态处理。当前批次的缺失图片使用几何占位，后续可替换资源槽位而不改变页面语义。

`--page-demo` 只接受 `demo_named` 已登记的 ID。窗口中的 `PageUp` / `PageDown` 只切换 demo fixture，不改变正式页面状态合同。ops 的 `--demo` 同样限制在当前目录，因此传入 Windows runner 的不是任意命令文本。`assets/other/DP背包.png` 目前仍是背包视觉参考表，不作为单个物品图标直接加载。
