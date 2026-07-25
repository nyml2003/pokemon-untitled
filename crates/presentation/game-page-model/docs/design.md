# 设计

## 状态边界

`PageState` 只持有 `PlayerRoute` 中的页面、选中项和数量。金钱、背包、队伍、存档可用性和商店价格每次通过 `project_page` 从 `ProductSnapshot` 与 `ThinSliceContent` 读取。

页面 reducer 不执行 `ProductCommand`。购买只产生 `PageEffect::SubmitProduct`，保存只产生 `PageEffect::RequestSave`；应用层负责执行请求、获得新快照并重新投影。

## Demo 边界

`PageDemo` 使用稳定 `PageDemoId` 和纯页面初始路由。`PageDemoContext::standard` 构建内容与只读快照，随后通过同一个 `project_page` 获得页面模型。该过程不依赖 renderer 或 runtime。

目录测试要求当前 `PlayerPage` 的每个枚举值各有一个 demo，且 ID 不重复。新增页面时必须同步增加 demo 和覆盖测试，才能接入任何 binary 路由。
