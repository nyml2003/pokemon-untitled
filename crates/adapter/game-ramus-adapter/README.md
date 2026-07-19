# game-ramus-adapter

## 职责

将玩家和 AI 的 Ramus DSL 输入注册、授权、校验并路由为 `RoutedIntent`。

## 状态契约

Router 不保存游戏状态、不读写文件，也不执行游戏命令。每个 provider 调用只返回已校验的 intent；玩家和 AI 都只能通过已注册的 intent 进入编排层。

## 公开 API

`GameRamusRouter::route` 接受换行分隔的 Ramus 调用，返回 `GameCommand` 或 `Save` 意图。已注册范围包含 NPC 交互、移动、warp、遭遇、战斗结算、购买和存档。

## 设计

注册表、授权与 provider 输出的边界见 [docs/design.md](docs/design.md)。

## 验证

`ops format --check`、`ops lint`、`ops test --suite all`、`ops docs check`。
