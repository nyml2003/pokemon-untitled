# Rust crate 模块结构

## 设计目标

`lib.rs` 是 crate 的稳定入口，不是所有实现的容器。
按业务模型、状态机、错误或协议等长期职责划分源文件。
不要按单次改动、测试名称或临时代码路径划分模块。

## 导出面

使用私有模块和显式根导出：

```rust
mod actor;
mod error;

pub use actor::{Actor, ActorId};
pub use error::DomainError;
```

外部调用方依赖 `crate_name::Actor`，不依赖 `crate_name::actor::Actor`。
拆分前先搜索 workspace 中的导入路径。
保留已有根路径，或在明确的破坏性变更中更新所有调用方。

## 可见性

- `pub` 只用于 crate 的调用方需要构造、匹配或调用的 API。
- `pub(crate)` 只用于同一 crate 内多个模块共享的实现细节。
- 私有成员用于维持聚合状态和模块内部不变量。

不要把字段改为 `pub` 来绕过模块边界。
跨模块需要读取时，优先使用已有访问器。
跨模块需要修改时，优先把转换收口在拥有状态的类型上。

## 测试与验证

测试可以保留在其状态机或模型所在模块。
拆分后，先运行 `cargo fmt --all -- --check` 和 `cargo test -p <crate>`。
被其他 crate 使用的公共 crate 还应运行 `cargo check -p <direct-dependent-crate>`。
影响多条依赖路径时运行 `cargo test --workspace`。
