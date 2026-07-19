# ops lint 严格 panic API 债务

`ops lint` 已对生产代码拒绝 `unwrap`、`expect`、`panic`、`todo`、`unimplemented` 和 `unreachable`。默认 Clippy 阶段已经通过。严格阶段仍有历史调用点需要清理。

## 当前债务

| 范围 | 问题 | 收敛方式 | 完成标准 |
| --- | --- | --- | --- |
| `game-session` | 对战快照根据显示中的宝可梦 ID 查找精灵槽位；当前无错误返回的 `snapshot()` 不能传播查找失败。 | 将 `GameSession::snapshot()` 和展示调用链改为可返回 `GameError::SpriteSlotMissing`，移除临时首槽位回退。 | 未知宝可梦 ID 不会被错误地渲染为首只精灵，且保持严格 lint 通过。 |
| `game-ui` console | console 默认构造会保存 Ramus adapter 初始化失败，但打开 console 的命令列表 API 尚不能显示该诊断。 | 让 `GameConsole::entries` 返回诊断结果，并由宿主传给 console 状态显示。 | adapter 初始化失败时 UI 显示具体诊断，而不是仅显示“没有可用指令”。 |
| 全局集合访问 | `clippy::indexing_slicing` 在现有 domain 和 foundation 代码中发现大量索引与切片访问；直接全局拒绝会使主 lint 长期失绿。 | 先在 `punctum-grid`、battle 和 map 的值对象中提供可证明边界的查询 API，再按 crate 纳入严格 lint。 | 不再依赖裸索引处理可变或外部输入，且全工作区启用该 lint 后通过。 |

## 推进规则

1. 每次只处理一个 crate 或一个完整的错误模型。
2. 先保留或补齐 `Result`、`Option` 和领域错误，再删除 panic API。
3. 状态机异常不能用静默默认值掩盖。
4. 每批执行 `ops format` 和 `ops lint`。涉及行为变化时执行 `ops test`。
5. 条目完成后从本文删除，并在对应实现或测试中保留可验证的失败路径。
