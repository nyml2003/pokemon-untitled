# game-page-model

`game-page-model` 是玩家页面的渲染无关模型层。它从 `ThinSliceContent` 和 `ProductSnapshot` 投影页面数据，保存路由与选择，并将用户意图转换为路由变化或类型化应用请求。

它不读取按键、创建窗口、访问文件系统，也不生成 `UiTree`、GPU 帧或终端输出。

当前公开入口包括：

- `PageState`：世界、暂停、商店和保存确认的局部路由状态与 reducer。暂停子页携带稳定的成员、物品、背包分类或全国图鉴编号选择。
- `BagFilter`：背包的全量、药品、关键物品和杂项筛选值；分类切换不改变物品 ID 选择合同。
- `project_page`：从权威内容和产品快照生成 `PageModel`。
- `page_demos`、`PageDemoContext`：标准小镇与真实教授赠礼快照的稳定 demo 目录。

维护边界和测试约束见 [docs/design.md](docs/design.md)。
