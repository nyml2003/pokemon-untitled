# 当前架构

> 分类：现状；最后核对：2026-07-20
> 依据：根 `Cargo.toml`、各 crate 的 `Cargo.toml` 和 `AGENT.md`

## Workspace

workspace 当前有 44 个 package。分组与职责如下。

| 区域 | 数量 | 职责 |
| --- | ---: | --- |
| `foundation` | 6 | 通用模型和工具，包括 Punctum 与 Ramus。 |
| `domain` | 9 | 战斗、数据、地图、世界和叙事规则。 |
| `application` | 6 | 用例编排、会话和编辑器状态转换。 |
| `presentation` | 9 | UI 状态、场景投影、渲染计划和资产请求。 |
| `adapter` | 8 | 文件、导入、Ramus、终端和 WGPU 等外部能力。 |
| `runtime` | 4 | 原生程序装配和平台事件循环。 |
| `quality` | 2 | 端到端与环境验证。 |

依赖方向固定为：

```text
runtime -> adapter -> presentation -> application -> domain -> foundation
```

领域、用例和表现层不依赖文件系统、窗口、GPU 或平台 API。真实资产读取与 native 提交留在 adapter/runtime。

## 运行入口

| 入口 | 位置 | 用途 |
| --- | --- | --- |
| `game-host` | `crates/runtime/game-host` | 单机桌面游戏。 |
| `map-editor` | `crates/runtime/map-editor` | 原生地图编辑器。 |
| `map-editor-cli` | `crates/runtime/map-editor-cli` | 地图项目命令行操作。 |
| `tile-editor` | `crates/runtime/tile-editor` | 瓦片语义编辑器。 |
| `game-data-import` | `crates/adapter/game-data-import` | 游戏数据导入 CLI。 |

`game-session` 是游戏规则状态的权威所有者。`map-editor-core` 是地图编辑状态机。表现层将快照投影为视图和帧计划；运行时负责平台事件、路径、外部能力装配与提交。

## 游戏帧流

```text
输入事件
  -> PresentationState / GameCommand
  -> GameSession::transition
  -> GameSnapshot
  -> project_scene
  -> SceneFrame
  -> NativeTarget::present
```

`PresentationState` 保存菜单、焦点和动画等表现瞬态状态，不能直接改变 `GameSession` 的内部规则状态。`project_scene`、UI tree 和帧计划应能在没有窗口、GPU 或真实资产的测试中构造。

## 资产与平台

`assets/` 与 `maps/` 属于 workspace，不属于单个 crate。内嵌只读游戏数据可以留在 `game-data`；路径解析、目录扫描和文件读取属于 adapter/runtime。

Windows 和 macOS 是 `game-host` 的原生构建与试玩平台。WSL 用于编辑、格式化和无窗口测试。Web 是未来的独立 runtime/adapter，不得把浏览器 API 引入核心分层。
