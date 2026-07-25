# game-page-demo

## 职责

`game-page-demo` 创建一个原生窗口，选择已登记的 `PageDemo`，把键盘语义和有限的鼠标悬停、点击转换为 `PageIntent`，并呈现已 resolve 的页面 frame。页面状态由 `game-ui::PageUiState` 管理，输入设备不会进入 `game-page-model`。

## 状态契约

它持有只读 `PageDemoContext` 与页面局部 `PageState`。页面只声明语义资源槽位，demo 壳绑定当前仓库已有的角色和宝可梦 PNG；没有对应素材的页面元素继续使用几何占位。`PageEffect::SubmitProduct` 和 `PageEffect::RequestSave` 只会显示反馈；该 crate 不提交产品命令，也不写入存档。

## 公开入口

不带参数时打开 `world-starting-town`。使用 `--page-demo <PageDemoId>` 直接打开指定 fixture，并拒绝未登记的 ID：

```text
cargo run --bin game-page-demo -- --page-demo party-single-member
```

窗口运行中使用 `PageUp` / `PageDown` 在已登记 fixture 间切换。这是 demo 壳的页面浏览快捷键，不属于正式游戏输入合同。

当前可用的世界 fixture 包括 `world-starting-town` 和 `world-starting-down`。

通过 ops 在 Windows 原生端打开受限 demo：

```text
ops run game-page-demo --demo party-single-member
```

## 设计

见 [设计](docs/design.md)。

## 验证

```text
ops format --check
ops lint
ops test
```
