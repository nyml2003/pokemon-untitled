# Workspace 顶层布局评估

> 分类：评估报告；核对日期：2026-07-20
> 范围：仓库根目录的直接子项；不修改现有目录、路径或 Git 配置。

## 结论

业务源码、项目内容和 workspace 级配置留在顶层是合适的。

不应作为仓库内容长期保留的只有本机生成物和本机私有配置：`target/`、`.venv/`、`.direnv/`、`tools/__pycache__/` 与 `ops.local.json`。它们当前均未被 Git 跟踪，符合这一判断。`target/` 约为 12 GB，是唯一需要优先回收的空间项。

`assets/`、`maps/`、`narrative/` 虽然不属于 Rust crate，但属于整个游戏的共享输入。当前不应搬入 `crates/` 或另设深层目录。这样做会破坏编译期嵌入、运行时默认路径和导入测试，收益不足以覆盖迁移成本。

## 评估标准

顶层项目应满足至少一项条件：

1. Cargo、Nix、Direnv、编辑器或 Git 需要从仓库根发现它。
2. 它是多个 crate、运行时或工具共同使用的输入。
3. 它定义整个 workspace 的源码、文档、工具或项目级约束。

不满足上述条件，且内容由本机命令生成、含本机路径或可由声明重新生成的项目，不应作为仓库内容保留。

## 顶层项目清单

| 项目 | 当前状态 | 判断 | 原因与处理建议 |
| --- | --- | --- | --- |
| `Cargo.toml`、`Cargo.lock` | 已跟踪 | 保留顶层 | 定义 44 个 package 的 workspace、共享依赖和锁定解析结果。Cargo 以根目录为 workspace 边界。 |
| `crates/` | 已跟踪 | 保留顶层 | Rust 源码的主集合，且已按 foundation、domain、application、presentation、adapter、runtime、quality 分层。 |
| `assets/` | 已跟踪，约 80 MB | 保留顶层 | 全局游戏内容。`game-data`、`game-host`、地图编辑器和导入测试均直接引用其路径或在编译期嵌入其内容。 |
| `maps/` | 已跟踪，约 3.5 MB | 保留顶层 | 跨运行时共享的地图项目输入。`game-host`、`map-editor` 和 `map-editor-cli` 以仓库根下的 `maps/` 为默认位置。 |
| `narrative/` | 已跟踪 | 保留顶层 | 游戏叙事脚本输入。`game-host` 在编译期通过 `include_str!` 嵌入 `narrative/demo/` 中的脚本。 |
| `fixtures/` | 已跟踪 | 保留顶层 | 跨 crate 的稳定测试数据。当前规模很小，独立目录比散落到各 crate 更易发现和复用。 |
| `tools/` | 已跟踪 | 保留顶层 | workspace 运维入口。Nix 开发环境以 `python -m tools.pokemon_ops` 启动 `ops`。 |
| `docs/` | 已跟踪 | 保留顶层 | 跨 crate 文档的唯一合理入口；crate 专属文档仍留在各 crate 内。 |
| `AGENT.md` | 已跟踪 | 保留顶层 | 项目级约束，需在进入任一 crate 前被发现。 |
| `.cargo/` | 已跟踪 | 保留顶层 | Cargo 配置按 workspace 根生效。 |
| `.codex/` | 已跟踪 | 保留顶层 | 项目专用 agent skill 与工作约束，服务整个仓库。 |
| `.vscode/` | 已跟踪 | 保留顶层 | workspace 级编辑器设置；内容目前只有 Rust 分析器 sysroot 配置。 |
| `.envrc`、`flake.nix`、`flake.lock` | 已跟踪 | 保留顶层 | Direnv 和 Nix 的标准根发现位置，定义可复现开发环境。 |
| `.gitattributes`、`.gitignore` | 已跟踪 | 保留顶层 | Git 规则必须覆盖整个仓库。 |
| `ops.local.example.json` | 已跟踪 | 保留顶层 | `ops.local.json` 的公开模板；工具按根目录文件名读取本机配置。 |
| `ops.local.json` | 未跟踪，已忽略 | 不纳入仓库内容；物理位置可留在顶层 | 含 Windows 镜像和本机 Python 路径。`tools/pokemon_ops/adapters/local_config.py` 固定从当前仓库根读取它。不要提交或迁入版本化目录。 |
| `target/` | 未跟踪，已忽略，约 12 GB | 不纳入仓库内容；可清理 | Cargo 构建产物，可完整再生。它留在根目录是 Cargo 默认行为，不是源码布局问题。磁盘紧张时直接删除该目录后重建。 |
| `.venv/` | 未跟踪，已忽略，约 3.2 MB | 不纳入仓库内容；可清理或外置 | Python 虚拟环境是本机依赖缓存。它不应提交；若希望根目录只保留项目文件，可由外部工具目录承载。 |
| `.direnv/` | 未跟踪，已忽略，约 104 KB | 不纳入仓库内容；可清理 | Direnv 本机状态。根目录中的位置是工具默认行为，不应提交。 |
| `tools/__pycache__/` | 未跟踪，内容已忽略 | 不纳入仓库内容；可清理 | Python 字节码缓存。保留在 `tools/` 下不影响源码结构，但不应被版本控制。 |
| `.git/` | Git 元数据 | 保留原位，不纳入项目布局讨论 | Git 工作树的必需控制目录，不参与源码和交付物组织。 |

## 不建议现在执行的迁移

### 不要把 `assets/` 搬到单个 crate

它并不归属于单个 crate。当前至少有以下直接路径契约：

- `crates/domain/data/game-data/src/lib.rs` 使用 `include_bytes!` 嵌入游戏数据。
- `crates/adapter/game-data-import-core/src/lib.rs` 使用 `include_str!` 嵌入 PokeAPI 导入快照。
- `crates/runtime/game-host/`、`map-editor/`、`map-editor-cli/` 与 `tile-editor/` 从 workspace 根下解析资产路径。

迁入任一 crate 会使领域层持有文件布局，或要求多个 runtime 重复资产适配。两者都不符合当前分层。只有在引入稳定的 `AssetRoot` 配置边界，并删除这些相对路径和编译期嵌入后，才应评估重组。

### 不要把 `maps/` 与 `narrative/` 塞入 `assets/`

三者的生命周期不同：`assets/` 是图片、数据和导入材料；`maps/` 是可编辑地图项目；`narrative/` 是 DSL 源文件。合并只会让编辑器输入、许可证材料和运行时资源难以区分。顶层并列比深层混放更清楚。

### 不要把 `tools/` 移入 `crates/`

`tools/pokemon_ops` 是 Python 运维工具，不是 Rust package。Nix 的 `ops` 命令已以当前模块路径启动它。迁移会同时影响开发 shell、文档和自动化入口，不能视为普通整理。

## 建议动作

| 优先级 | 动作 | 完成标准 |
| --- | --- | --- |
| 高 | 定期清理 `target/` | 根目录不长期占用过期构建产物；下次构建可自动恢复。 |
| 中 | 在本机初始化脚本中明确 `.venv/`、`.direnv/` 与 `ops.local.json` 为私有状态 | 新成员不会误提交这些文件；`git status --ignored` 可确认它们仍被忽略。 |
| 低 | 为运行时引入可配置的资源根后，再评估内容目录迁移 | 所有资产、地图和叙事路径由单一边界提供；无 crate 使用 `../../../assets`、`../../../maps` 或 `include_*` 指向根目录。 |

本次只新增本报告。未删除 `target/` 或其他本机文件，避免改变当前开发环境。

## 证据

- 根 `Cargo.toml`：workspace 成员与共享依赖。
- `docs/v2/current/001-架构总览/README.md`：`assets/`、`maps/` 的 workspace 归属和资产边界。
- `flake.nix`：`ops` 对 `tools.pokemon_ops` 的模块入口。
- `tools/pokemon_ops/adapters/local_config.py`：`ops.local.json` 的根目录约定。
- `crates/domain/data/game-data/src/lib.rs`、`crates/adapter/game-data-import-core/src/lib.rs`、`crates/runtime/game-host/src/{map,narrative,sprites}.rs`、`crates/runtime/{map-editor,map-editor-cli,tile-editor}/src/`：共享内容目录的直接路径与嵌入引用。
- `.gitignore`：本机生成物与私有配置的忽略规则。
