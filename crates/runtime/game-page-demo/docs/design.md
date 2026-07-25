# 设计

## 状态边界

`PageDemoContext` 提供静态内容和只读产品快照，`PageState` 只保存当前页面路由与局部选择。点击命中已 resolve 的 `UiFrame<PageIntent>` 后，runtime 将 intent 交给 `PageState::transition`，再从同一快照重建 `PageModel`。

runtime 不执行 reducer 返回的 `PageEffect`。购买和存档请求只转为短反馈文字，因此 demo 不会修改产品状态、文件或地图世界。

## 呈现边界

壳只注册 `solid/white` 填充资源，并通过 `FramePlan::from_ui_frame` 将 `game-view::project_page_model_with_notice` 的输出交给 `NativeTarget`。页面模型投影不产生资源图像命令；这个 crate 不加载地图、角色或其他游戏资源。

## 输入与选择

鼠标点击使用 `UiFrame::hit_action` 获取最上层的类型化 action。窗口、坐标与鼠标事件不进入 `game-page-model`。

`--page-demo` 只接受 `demo_named` 已登记的 ID。ops 的 `--demo` 同样限制在当前目录，因此传入 Windows runner 的不是任意命令文本。
