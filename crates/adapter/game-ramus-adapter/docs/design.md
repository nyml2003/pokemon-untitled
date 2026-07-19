# 设计

`GameRamusRouter` 为每个玩家和 AI 输入创建同一份 Ramus catalog 与最小能力授权。

Provider 不持有 `GameState`。它只将已经通过 Ramus schema 验证的请求编码为 `RoutedIntent`，再由 router 解码为类型化游戏命令。

`Save` 是与状态命令并列的 intent。运行时入口消费它并进行文件读写，因此 router 不依赖文件系统、时钟、窗口或随机数。
