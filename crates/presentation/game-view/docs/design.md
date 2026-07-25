# 设计说明

`lib.rs` 只定义稳定根导出。
`projection` 统一保存不依赖窗口、GPU 或资源文件的视图投影实现及其测试。

`GameView` 的图层顺序是从地图到角色、HUD 和控制台。
页面投影返回 `punctum-ui` 的数据树，而非平台控件。
资源键只描述需要的资源，不加载或解码资源。

`project_page_model` 是独立任务面板的投影入口。它只接受渲染无关的 `PageModel`，并且不产生地图层、角色层或图像命令；页面 demo 因而能在无地图 renderer 的测试中验证布局与类型化 action。
