# Ramus

Ramus 把结构化 application API 投影为一棵类型安全的能力树。`Ramus` 是拉丁语“枝条”，对应从根路径展开的 capability branch。

核心 crate 名为 `ramus-core`，位于根 workspace 的 `crates/foundation/ramus`。

## v1 语法

```text
/<node-path> <method> [name=value | positional]...
```

示例：

```text
/battle/turn submit move=thunderbolt target=opponent:1
/battle/turn legal
/battle/state get
/dev/battle seed value=42
/bigFunc1 smallFunc1 arg1 arg2
```

路径是虚拟能力树中的 file-like address，不是宿主文件系统路径。方法是该节点暴露的 typed operation。这个拆分沿用 object path + method + schema 的常见 RPC/组件模型；discover 与 complete 从同一份 capability-filtered schema 生成。

v1 支持：

- 绝对节点路径和方法名。
- 换行分隔的顺序调用。
- 命名参数和位置参数。
- 字符串、`i64`、布尔值和枚举 schema。
- 双引号字符串及 `\"`、`\\`、`\n`、`\r`、`\t` 转义。
- capability-filtered discover、complete、read、write 和 invoke。
- 可配置的 ShellText 与 Agent PlanDraft 资源上限。

v1 不支持管道、重定向、变量、命令替换、分号语句或任意文件 IO。引号内的 shell 元字符只作为字符串数据。复杂编排应由 application API 或后续显式 plan 节点表达，不把宿主 shell 权限带入 Ramus。

## 边界

纯核心负责 parser、AST、typed value、schema、catalog、capability view、compiler、sealed plan 和执行前 preflight。它不读取共享可变状态，也不调用 provider。

`boundary/authorization.rs` 负责 principal 签发、授权状态、generation 和原子 `EffectPermit` 签发。管理端持有不可复制的 `AuthorizationService`。runtime 只持有 `AuthorizationChecker`，需要主动撤权的 application 回调只持有 `AuthorizationRevoker`。

能力查询必须在短生命周期 `AuthorizationSession` 内完成。`CapabilityView` 借用 session，不能 clone，也不能在 session 释放后继续使用。撤权会等待已有 session 结束；撤权完成后，旧权限不能再用于 discover、complete、seal 或 provider 状态探测。

`boundary/runtime.rs` 负责 provider 绑定和调用。`Provider::execute` 是 application 副作用进入 Ramus 的唯一端口。

`PlanDraft` 永远不可信。只有 `Compiler::seal` 可以构造 `TypedPlan`。`TypedPlan` 记录 principal、catalog generation、完整 schema、schema version、provider identity、effect 和 capability generation。每个 effect 执行前都重新签发单次消费 permit。

`parse()` 的本地默认上限为 64 KiB source、64 calls、每个 call 64 arguments。`Compiler::seal()` 还限制 64 KiB PlanDraft、单值 16 KiB、4096 个 value node 和 32 层嵌套。正式 host 应使用 `parse_with_limits()` 和 `Compiler::seal_with_limits()` 显式传入场景限制。

## 本地验证

```text
ops format --check
ops test --suite all
```

所有验证均从仓库根目录的 `ops` 入口执行。
