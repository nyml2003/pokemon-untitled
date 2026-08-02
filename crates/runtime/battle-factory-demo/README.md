# battle-factory-demo

## 职责

打开一个原生窗口演示对战工厂模式。它加载 `battle-factory` 会话与战斗精灵资源，把键盘输入转换为 `FactoryCommand`，并投影战斗或工厂菜单画面。

## 状态契约

持有 `FactorySession` 与只读 `FactorySnapshot`。对战阶段复用 `BattleUiState` 的输入语义，回放由真实时间计时推进；工厂菜单阶段用方向键与 A/B 操作。该二进制不读写存档。

## 公开入口

```text
cargo run --bin battle-factory-demo
```

默认使用随机种子与 `DEFAULT_TARGET_STREAK`（7 连胜）开始一轮。窗口内按键：

- 菜单/战斗：A 确认，B 返回；方向键选择。
- 交换选择：↑↓ 选己方槽位，←→ 选对手槽位，A 交换，B 跳过。
- 结果页：A 开始新一轮。

通过 ops 在 Windows 原生端打开：

```text
ops run battle-factory-demo
```

## 设计

见 [设计](docs/design.md)。

## 验证

```text
ops build battle-factory-demo
```
