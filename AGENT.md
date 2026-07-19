# 项目约束

这是一个 Rust workspace。修改前先读目标 crate 的 `Cargo.toml` 和 `src/`，再决定边界归属。不要根据目录名猜测职责。

## 语言

除代码、命令、路径和必要技术术语外，文档、注释、提交说明和协作沟通尽可能使用中文。

## Rust 注释

新增或修改 Rust 的 `//`、`//!`、`///` 注释前，读取 [`.codex/skills/rust-commenting/SKILL.md`](.codex/skills/rust-commenting/SKILL.md)。

- 详细的 rustdoc 类型、内容范围、换行和验证规则见 [`references/rust-comments.md`](.codex/skills/rust-commenting/references/rust-comments.md)。
- 每个完整句子必须写在同一注释行；不要在句中换行。

## Rust crate 结构与文档

拆分 Rust crate、收紧模块可见性或调整根导出面前，读取 [`.codex/skills/rust-crate-structure/SKILL.md`](.codex/skills/rust-crate-structure/SKILL.md)。

新增或修改 crate 的 README 和 `docs/` 设计文档前，读取 [`.codex/skills/rust-crate-documentation/SKILL.md`](.codex/skills/rust-crate-documentation/SKILL.md)。

## 当前分层

`crates/foundation/`

- 通用模型和纯工具：`punctum-grid`、`punctum-input`、`punctum-gpu`、`punctum-ui`、`ramus`。
- 不依赖任何游戏 crate、平台 API、文件系统或窗口库。

`crates/domain/`

- 游戏规则、数据模型、地图语义、叙事模型。
- 不读写文件，不创建窗口，不调用 GPU 或网络。
- 用类型区分 ID、坐标、命令和状态。正常失败返回 `Result` 或明确的错误枚举。

`crates/application/`

- 编排领域用例和状态转换。
- `game-session`、`world-application`、战斗 session 是游戏状态的权威所有者。
- 保持状态转换可复现。真实时间、窗口事件和文件加载由调用方提供，不进入领域规则。

`crates/presentation/`

- 将游戏快照投影为视图、UI 和 GPU 提交计划。
- `game-scene-view` 生成 `SceneFrame`，`game-native-plan` 生成 `FramePlan`。
- 可以依赖 domain、application 和 foundation；不得依赖 runtime、adapter、Winit、WGPU 或文件系统。
- 这里描述“画什么”，不描述“怎样提交到某个窗口”。

`crates/adapter/`

- 外部能力的实现层。
- `game-fs-assets` 负责本地资产读取。
- `punctum-wgpu` 负责 Winit surface、WGPU device 和 present。
- `game-native-target` 负责将 `FramePlan` 编码为 native WGPU submission，并处理 glyphon 文本。
- 平台或库返回的错误在此层或 runtime 层转换，不向 domain 泄露平台错误码。

`crates/runtime/`

- 可执行程序的装配层。
- `game-host` 负责 Winit 事件循环、文件路径、资产装配、输入归一化和调用 `NativeTarget`。
- runtime 不新增游戏规则；规则应进入 domain 或 application。

`crates/quality/`

- 端到端和环境验证工具。
- 不作为产品运行时依赖。`wslg-wgpu-clear-smoke` 仅用于复现 WSLg/WGPU 环境问题。

## 依赖方向

依赖只能从外向内：

```text
runtime -> adapter -> presentation -> application -> domain -> foundation
```

- 不要让 domain、application 或 presentation 反向依赖 adapter/runtime。
- 不要把 Winit、WGPU、`std::fs`、`std::net` 或平台句柄引入 domain/application。
- 新增外部能力时，先定义输入输出数据，再在 adapter 或 runtime 实现它。
- 不要为了复用方便跳过层级。确有共享模型需求时，放入合适的 foundation、domain 或 presentation crate。

## 游戏帧流

保持以下数据流。不要让 UI 或渲染层直接修改 `GameSession` 内部状态。

```text
输入事件
  -> PresentationState / GameCommand
  -> GameSession::transition
  -> GameSnapshot
  -> project_scene
  -> SceneFrame
  -> FramePlan
  -> NativeTarget::present
```

- 输入先归一化为 `punctum-input` 类型，再进入 UI 或 game session。
- `project_scene` 和 `FramePlan` 必须可在没有窗口、GPU 和真实资产路径的测试中构造和验证。
- 渲染失败只影响当前运行时；不要用渲染结果修改游戏规则状态。

## 平台规则

- 当前单机桌面入口是 `game-host`。
- Windows 和 macOS 应原生构建、运行和验证 `game-host`。它们使用现有 Winit/WGPU native 链路。
- WSL 用于编辑、格式化、静态检查和无窗口测试。不要把 WSLg 当作桌面试玩环境。
- Web 是未来的独立 runtime/adapter：复用 `game-session`、`game-scene-view` 和 `FramePlan`，替换文件系统资产读取和 native present 边界。不要为了 Web 把浏览器 API 引入 core。
- 不要在 Linux/WSL 中假设 Windows、macOS 或浏览器后端的行为；在对应平台运行验证。

## 资产与 IO

- `game-data` 的内嵌只读数据可以留在 domain；运行时路径、目录扫描和文件读取留在 runtime/adapter。
- 新增本地资产读取时，优先扩展 `game-fs-assets` 或新增同类 adapter。不要把 `std::fs` 扩散到 presentation/application。
- 为 Web 预留以字节或 `AssetRequest` 为中心的接口，不要把 `PathBuf` 放进游戏规则或视图模型。

## 修改与验证

- 先做最小改动。不要在功能修改中顺带重命名 crate 或重排无关模块。
- 新规则或新状态转换必须有单元测试，包含正常失败路径。
- 修改 presentation plan 时，测试 plan/编码契约，不依赖真实窗口截图作为唯一验证。
- 修改 WGPU/Winit/窗口行为时，在目标桌面平台实际运行；WSL 中至少执行编译和无窗口测试。
- 默认执行与改动范围匹配的检查：

```sh
cargo fmt --all -- --check
cargo test -p <受影响的 crate>
cargo check -p <受影响的 crate>
```

- 涉及共享 crate、依赖图或渲染提交边界时，补跑 `cargo test --workspace` 或说明未运行的原因。
- 保持 `#![forbid(unsafe_code)]` 的 crate 不引入 unsafe。需要 unsafe 时，先隔离到最窄的 adapter，并写明安全不变量。

## 错误与状态

- 预期失败使用结构化 `Result` 和错误类型；不要用 `panic!`、`expect` 或字符串匹配处理用户输入、资产缺失、状态冲突或平台失败。
- `expect` 仅用于已由构造器或本地不变量保证的条件。
- UI、CLI 和 runtime 负责格式化错误、写日志和退出；domain/application 只返回可判断的数据。
