# punctum-ui

## 职责

`punctum-ui` 定义与渲染器无关的像素 UI 树，并将其解析为有序绘制命令和命中区域。
它会在布局前校验结构 ID、动态 key、比例基数和逻辑画布尺寸。

## 状态契约

`UiTree::new` 确定性地分配自动 ID，并拒绝重复的显式 ID 或 key。
`UiTree::resolve` 是纯操作：相同的树和 viewport 必定得到相同的 `UiFrame`。
该 crate 不保留输入状态、不加载资源，也不提交渲染工作。

## 公开 API

调用方用 `UiStyle` 和 `UiContent` 构建 `UiNode` 树，再包装为 `UiTree`。
使用 `UiSize` 解析树，得到 `UiDrawCommand`、`UiHitRegion` 和带类型的 `UiActionHit`。
`UiBuildError` 表示无效的树数据，`UiLayoutError` 表示无法满足最小尺寸的布局。

## 设计

详见[设计说明](docs/design.md)。

## 验证

在 workspace 根目录运行 `ops test --suite all` 和 `ops format --check`。
