# game-page-demo

## 职责

`game-page-demo` 创建一个原生窗口，选择已登记的 `PageDemo`，把鼠标点击转换为 `PageIntent`，并呈现已 resolve 的页面 frame。

## 状态契约

它持有只读 `PageDemoContext` 与页面局部 `PageState`。`PageEffect::SubmitProduct` 和 `PageEffect::RequestSave` 只会显示反馈；该 crate 不提交产品命令，也不写入存档。

## 公开入口

不带参数时打开 `world-starting-town`。二进制只接受 `--page-demo <PageDemoId>`，并拒绝未登记的 ID。

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
