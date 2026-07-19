# 性能热点静态分析报告

> 分类：优化分析；核对日期：2026-07-20。
> 范围：`game-host` 游戏运行时、WGPU 提交链、地图编辑器、地图语义检查和战斗/世界核心。
> 结论等级：静态证据只能证明执行频率、遍历规模和分配边界；实际耗时必须在目标桌面平台采样确认。

## 结论

当前最值得优先测量和优化的是游戏重绘链，而不是世界规则或战斗规则。

一次 `RedrawRequested` 会从 `GameSnapshot` 重新投影场景，重新建立 `FramePlan`，重新生成实例字节，再上传 GPU。固定游戏画布为 `32 x 24`，因此每次游戏画面至少会处理 768 个网格实例；地图、角色、UI 图片和文本会继续增加实例、排序、分配与上传成本。动画、按键、输入法和每秒世界 tick 都可能触发这条路径。

第二优先级是地图编辑器。它已经只投影视区瓦片，但语义覆盖层仍在每次重绘扫描整张地图。当前区域图为 `72 x 56`，即 4032 格；地图尺寸扩大或鼠标连续移动时，这段全图扫描会直接进入交互延迟路径。

不建议先优化地图启动检查、全图语义检查或战斗算法。它们目前位于启动、保存或玩家动作等低频边界，且输入规模较小。应先取得帧阶段耗时、实例数、上传字节数和提交次数，再决定缓存或增量更新的具体形态。

## 分析方法与限制

### 静态证据

- 检查了游戏帧流：`game-host -> game-scene-view -> game-native-plan -> game-native-target -> punctum-wgpu`。
- 检查了地图编辑器重绘、地图投影、地图语义校验和保存路径。
- 统计 Rust 源码共 54,796 行。该数字包含测试，不能代表运行时成本。
- 当前 `maps/verdant-route/` 有 11 个区域图；每个区域图为 `72 x 56`。游戏画布为 `32 x 24`，地图图块跨度为 `2 x 2`，正常视区约为 `16 x 12` 个地图格，像素位移时会增加少量 overscan。
- 运行了 `ops check --json`，结果通过。运行了 `ops lint --json`，但工作区现有的 `battle-session` API 迁移不一致导致检查失败。本报告未修改 Rust 源码，且不能据此给出可运行基线的性能结论。

### 不包含的证据

- 没有目标 Windows/macOS 原生运行数据，也没有 GPU 时间戳。
- 没有 P50/P95/P99 帧时间、内存分配次数、驱动提交耗时或实际 NPC/图层密度。
- 当前工作树有未提交变更；代码位置和行为以本次核对时的工作树为准。

## 热点矩阵

| 优先级 | 位置 | 静态事实 | 规模模型 | 判断 | 首先测什么 |
| --- | --- | --- | --- | --- | --- |
| P0 | 游戏帧计划与实例上传 | 每次重绘都会从 `GameView` 新建网格、图片、文本标签和 `SubmissionPlan`；随后编码实例并写入 GPU buffer。 | `O(768 + I + L)` CPU 遍历；实例上传约 `64 x (768 + I)` 字节，`I` 为图片实例数，`L` 为标签数。 | 高频且无跨帧复用，是最可能的 CPU 分配和上传热点。 | `project_scene`、`FramePlan`、实例数、上传字节数、`present` 的 CPU 时长。 |
| P0 | 重绘触发频率 | `RedrawRequested` 进入完整投影和提交；表现动画、按键、世界事件和窗口变更都会请求重绘。世界时钟每秒 tick 一次，动画可更频繁。 | 每次触发均执行完整帧链。 | 不需要连续空转，但动画期间的完整重建仍会放大 P0 成本。 | 每秒 redraw 次数，按触发源分桶，动画期与静止期分开看。 |
| P1 | 多 pass 的 WGPU 提交 | `FramePlan::compose` 保留 base 与 overlay 两个 pass；WGPU 运行时对每个 pass 分别创建 encoder 并 `queue.submit`。 | `O(P)` encoder 和 queue submit，`P` 为 pass 数。 | 控制台、战斗 UI、地图编辑器 chrome 会提高 `P`。驱动开销需原生采样确认。 | 每帧 pass 数、submit 数、每 pass 实例数和 CPU 编码时长。 |
| P1 | 文本整形 | 每个 pass 的每个标签都会新建文本 buffer，设置文本并执行 Advanced shaping。 | `O(L)` buffer 分配和整形；长文本/多标签会放大。 | UI、宝可梦图鉴和编辑器面板更可能受影响；目前无缓存。 | 标签数、字符数、文本 prepare/encode 耗时。 |
| P1 | 地图编辑器语义覆盖层 | 工作台投影先做视区地图渲染，但 `project_semantics` 对完整 `width x height` 双循环，再裁剪非视区格。 | `O(W x H)`；当前为 4032 格/重绘。 | 鼠标移动、画笔和窗口重绘都会走这条路径，地图扩大后风险直接上升。 | `project` 总耗时、`project_semantics` 耗时、实际可见格与扫描格比值。 |
| P2 | 世界角色投影 | 每次世界重绘克隆所有 actor 并排序，再筛除视区外角色。 | `O(A log A)`，`A` 为角色数。 | 当前 NPC 数量可能很小；开放区域和大量动态实体时会放大。 | actor 数、可见 actor 数、排序耗时。 |
| P2 | 地图语义 lint | 语义 lint 遍历所有格、所有图层、8 邻居和图样规则，并构建定义表。它在游戏启动和编辑器保存时调用。 | 约 `O(W x H x (层数 + 8 + 图样规则))`。 | 低频但可能拉长加载/保存。当前不在每帧链。 | 启动/保存时长、诊断数、每区域图耗时。 |
| P3 | 世界与战斗领域逻辑 | 世界 actor 查询使用线性查找；战斗按事件序列生成回放步骤。世界 tick 为 1 秒一次，队伍通常为 6。 | 世界移动约 `O(A)`；战斗约 `O(E)`。 | 现有频率和规模不足以优先优化。 | 大量 NPC 或长事件回放的专门压测后再决定。 |

## 已确认的主链

```text
输入/动画/世界 tick
        |
        v
game-host::redraw
        |
        v
GameSession::snapshot + project_scene
        |
        +--> project_map：仅遍历视区瓦片，当前约 16 x 12
        |
        v
FramePlan::from_game_view / from_ui_frame
        |
        +--> 新建 cells、images、labels 和实例列表
        +--> 图片按 z-index 排序
        +--> 编码每个实例为 64 字节
        |
        v
NativeTarget::present
        |
        v
GpuRuntime：每个 pass 上传、建 encoder、draw、submit
```

`project_map` 已经按可见范围遍历，而不是遍历整张 `72 x 56` 地图。这是正确的现有优化。不要将地图渲染改成全图缓存后每帧完整合成；应保持视区裁剪，并只降低视区内容的重复构造与上传。

## 关键证据

### 1. 每次游戏重绘重建投影与计划

`crates/runtime/game-host/src/main.rs` 的 `redraw` 依次取得快照、调用 `project_scene`、建立 `FramePlan`、调用 `NativeTarget::present`。同文件的输入处理、表现状态推进和世界时钟会请求重绘；`WORLD_LOGIC_TICK` 为 1 秒。

`crates/presentation/game-native-plan/src/lib.rs` 的 `FramePlan::from_game_view`：

- 分配完整网格 `cells`。
- 遍历每个视图 layer，复制 surface cell，收集图片，克隆标签文本。
- 将结果交给 `plan_composite`。

`crates/foundation/punctum-gpu/src/plan.rs` 的 `plan_composite` 再次遍历整张 surface，将每个格转为 `InstanceData`；图片被收集为新 `Vec` 并按 z-index 排序。`crates/foundation/punctum-gpu/src/encoding.rs` 将每个实例重新编码为 64 字节 `Vec<u8>`。

### 2. 覆盖层会增加提交次数

`FramePlan::compose` 只是把 overlay pass 追加到 pass 列表。`crates/adapter/punctum-wgpu/src/runtime.rs` 对每个 pass 执行上传、uniform 写入、command encoder 创建、render pass 和 `queue.submit`。这不是功能错误，但控制台、战斗 UI 和地图编辑器的 map/chrome 双 pass 会把 CPU 与驱动边界工作增加为多次。

### 3. 文本没有跨帧缓存

`crates/adapter/game-native-target/src/lib.rs` 的 `TextGpu::encode` 对每个 `NativeTextLabel` 新建 `Buffer`，设置文本并进行 Advanced shaping。文本内容、字体大小和 bounds 在静止 UI 中通常稳定，因此这是适合后续按脏标记缓存的候选；先以标签数和耗时确认收益。

### 4. 编辑器有一个确定的全图交互扫描

`crates/presentation/map-editor-view/src/workbench.rs` 的 `project_semantics` 遍历完整地图，然后才用 `MAP_RECT` 过滤不可见格。相同模块的 `project_map` 已按可见 tile range 工作，因此可将语义覆盖层改成复用相机和视区范围。这个改动不改变语义，只将循环边界从全部地图收窄到可见地图格。

### 5. 全图语义 lint 目前属于正确的低频边界

`crates/domain/map/map-tile-semantics/src/catalog.rs` 的 `lint` 会扫描全图、图层、邻居与图样规则。`game-host` 在加载区域时调用它，地图编辑器在保存前调用它。这里的正确优化方向是报告耗时、缓存静态 definition lookup，或在编辑器中按修改区域增量预览；不能因为性能而取消保存和启动的完整校验。

## 优化顺序

### 阶段 0：先建立可比较的基线

在不改变游戏规则的前提下，增加可开关的帧指标。指标应归属运行时和 adapter，不能把计时、WGPU 或窗口类型带入 domain/application。

| 指标 | 建议埋点边界 | 目的 |
| --- | --- | --- |
| `redraw_count_by_reason` | `game-host` 的按键、动画、tick、resize、显式 request 路径 | 找出无效或过密重绘。 |
| `scene_projection_us` | `project_scene` 调用外层 | 分离场景投影成本。 |
| `frame_plan_us`、`instance_count`、`upload_bytes` | `FramePlan` 与 `SubmissionPlan` 建立处 | 验证 CPU 构造和传输量。 |
| `pass_count`、`submit_count` | `NativeTarget::present` / `GpuRuntime` | 验证 overlay 造成的提交放大。 |
| `text_label_count`、`text_encode_us` | `TextGpu::encode` | 判断是否需要文本缓存。 |
| `editor_visible_cells`、`editor_scanned_cells`、`editor_projection_us` | 地图编辑器 `project` | 验证视区优化的真实收益。 |

在目标 Windows/macOS 原生运行时记录 30 秒静止、行走动画、打开控制台、战斗 UI、地图编辑器拖动四组场景。每组输出 P50/P95/P99 CPU 阶段时长和每帧提交计数。GPU 时间戳可以作为可选能力接入；不应在每帧同步等待 GPU 完成。

### 阶段 1：减少重复帧构造

1. 为 `GameSnapshot`、表现状态和 viewport 建立明确的 render dirty key。
2. 静止帧复用上一次的 `FramePlan`；只有动画帧、输入后的状态变化、窗口尺寸变化或资源变化才重建受影响部分。
3. 先缓存稳定的地图 base pass；角色、对话、光标和 UI 保持独立的动态 pass。
4. 保留现有纯 `project_scene` 和 `FramePlan` 合约。缓存应在 runtime/presentation 边界实现，缓存失效条件必须可测试。

完成标准：静止状态不重复构造完整地图实例；行走时只重建角色和必要覆盖层；状态切换后画面与当前测试快照一致。

### 阶段 2：降低 GPU 上传与提交边界成本

1. 复用实例字节缓冲或直接写入预分配 staging buffer，避免每帧创建编码 `Vec<u8>`。
2. 对稳定 grid 使用现有 `SubmissionMode::Delta` / `plan_patch` 能力，只上传变化 span。必须在 resize、atlas 变化和 surface 尺寸变化时安全回退到 `Replace`。
3. 评估把同一目标上的兼容 pass 合并到一个 command encoder 和一次 queue submit。不要为合并破坏不同 viewport、scissor、文本 overlay 的绘制顺序。

完成标准：指标能显示动态帧上传字节低于全量实例字节，且 pass 合并前后截图/plan 契约一致。

### 阶段 3：收紧地图编辑器的交互复杂度

1. 将 `project_semantics` 的循环范围替换为当前相机的可见行列范围，含一格安全边界。
2. 语义诊断的实时预览只重算被编辑格及其 8 邻居；保存时继续执行完整 `lint`。
3. 保存时保留 `MapProject::validate` 和完整语义 lint，不将性能缓存作为正确性来源。

完成标准：扫描格数与可见格数接近，不随整图面积线性增长；保存的完整校验仍能拒绝跨区域规则错误。

### 阶段 4：只在数据规模证明后优化领域查询

当单区域动态 actor 达到数百个或世界 tick 被提升到更高频率时，再将 `World` 的 actor 位置查询改为位置索引或稳定绘制顺序缓存。战斗侧仅在长回放或 AI 批量模拟出现证据时再优化事件分配。

## 风险与取舍

| 方案 | 收益 | 主要风险 | 防护 |
| --- | --- | --- | --- |
| 帧计划缓存 | 降低静止帧 CPU 分配和图层重建 | 脏标记遗漏导致旧画面 | 用快照、viewport、表现状态构成完整 cache key；写缓存失效测试。 |
| Delta 上传 | 降低带宽和编码成本 | buffer 容量、resize 与 pass 切换时内容不同步 | 尺寸变化强制 Replace；保持现有 DeltaGridMismatch 保护。 |
| 合并 submit | 降低驱动边界次数 | 渲染顺序、scissor 或 glyphon overlay 改变 | 先用双 pass 的视觉回归和 plan 合约测试锁定顺序。 |
| 编辑器增量语义 | 降低画笔/鼠标交互延迟 | 图样和邻居规则跨出增量范围 | 编辑时扩展到 8 邻居及图样影响范围；保存时全量 lint 兜底。 |

## 暂不作为热点的部分

- 地图加载、组合和语义检查：发生在启动和保存，不在每帧路径。内容包有 7,729 个文件、约 25 MB，但运行时实际解码集合需要单独量测，不能由仓库总量推断启动热点。
- `project_map`：已经按可见 tile range 裁剪。当前约处理 192 个可见地图格，而不是 4032 个整图格。
- 世界 actor 的线性查找与排序：有扩展风险，但当前 tick 频率低，且没有 actor 规模数据。
- 战斗规则与回放：队伍大小受限，事件序列通常短。先测 UI、文本与渲染提交。

## 建议的验收门槛

在目标桌面平台以 60 Hz 为体验目标时，先看 P95：CPU 侧 `scene_projection + frame_plan + present` 应保留足够预算给 GPU 与系统事件，推荐先以 8 ms 作为调查线、16.7 ms 作为不可长期超过的整帧线。该门槛用于发现回归，不应替代不同机器上的实际基线。

每次优化应至少同时证明以下三点：

1. 对应场景的 P95 指标下降，且没有把成本转移到另一个阶段。
2. `FramePlan`、Delta/Replace 和渲染顺序的现有契约测试保持通过。
3. Windows/macOS 原生运行的视觉结果和输入响应没有退化。

## 证据索引

- `crates/runtime/game-host/src/main.rs`：重绘、动画、世界 tick 和事件循环唤醒。
- `crates/presentation/game-scene-view/src/lib.rs`：场景投影、32 x 24 画布、地图相机。
- `crates/presentation/map-render/src/lib.rs`：可见地图范围和图层图片生成。
- `crates/presentation/game-native-plan/src/lib.rs`：`GameView`/UI 到 `FramePlan` 的完整构造。
- `crates/foundation/punctum-gpu/src/{plan,encoding}.rs`：实例规划、排序与 64 字节编码。
- `crates/adapter/punctum-wgpu/src/runtime.rs`：per-pass upload、encoder、draw 与 submit。
- `crates/adapter/game-native-target/src/lib.rs`：每标签文本 buffer 与 Advanced shaping。
- `crates/runtime/map-editor/src/main.rs`、`crates/presentation/map-editor-view/src/workbench.rs`：编辑器双 pass 和全图语义覆盖层。
- `crates/domain/map/map-tile-semantics/src/catalog.rs`、`crates/domain/map/map-project/src/lib.rs`：全图 lint 和保存验证。
- `maps/verdant-route/*.json`：当前区域图尺寸与内容规模。
