# 固定地图网格与 3x3 预加载窗口

> 状态：第二阶段已完成最小 host 接入

## 结论

开放世界使用固定大小的地图块网格。每个可进入的野外地图都是 `72x56` 个 tile。运行时始终准备当前地图和八个直接相邻地图，形成稳定的 `3x3` 加载窗口。

空坐标不是缺失文件错误。它是 `EmptyChunk`：渲染为统一背景，且不可进入。可行走的海、道路或草地必须是实际地图。

## 静态模型

```text
WorldProject
  initial: WorldChunkCoord
  maps: WorldChunkCoord -> MapProject

WorldChunkCoord
  x: i32
  y: i32
```

单张 `MapProject` 不保存相邻文件路径。它只保存自己的 ID、格子、人物和静态事件。世界坐标决定野外相邻关系：从 `(x, y)` 向右离开时，目标总是 `(x + 1, y)`。

`WorldProject` 校验：

- 地图坐标唯一。
- `MapProjectId` 唯一。
- 初始坐标存在。
- 每张地图通过既有 `MapProject::validate`。
- 每张地图尺寸都是 `72x56`。

这项尺寸约束只适用于野外 `WorldProject`。`MapProject` 本身仍可用于地图编辑器 fixture、室内地图或其他非网格用途。

## 加载窗口

```text
(-1,-1)  (0,-1)  (1,-1)
(-1, 0)  (0, 0)  (1, 0)
(-1, 1)  (0, 1)  (1, 1)
```

中心是当前地图。返回顺序固定为从左上到右下。host 和渲染层据此加载图片、地图投影和可见角色；空槽不请求地图文件或地图资源。

玩家跨边界后，目标块提升为中心。窗口只新增一行或一列，并回收离开窗口的重型渲染数据。访问过的地图运行时状态应保留在内存 `MapStateCache`；第一阶段不保存到磁盘。

## 运行时规则

- 当前地图接受玩家输入，并推进其 NPC 和脚本。
- 已加载相邻地图默认不推进 tick。需要远景活动时，再将其纳入明确的世界 tick 策略。
- `EmptyChunk` 阻止越界移动。
- 门、楼梯和洞穴不使用网格邻接。后续以 `Portal` 指向 `WorldChunkCoord + entry`。
- UI 只响应后续的 `MapWindowShifted` 事件做镜头与过渡，不决定目标坐标。

## 已完成

1. 新增纯 `world-project` crate，提供坐标、标准尺寸、注册表校验和 `3x3` 窗口。
2. 使用现有 `72x56` demo map 作为标准尺寸基线。
3. 为空槽、重复坐标、重复地图 ID、缺失初始地图和非标准尺寸编写内存测试。
4. 增加 `maps/verdant-route/world.json`。它只属于 runtime 文件适配器，保存坐标和相对地图文件名。
5. `game-host` 读取该清单，加载十张地图，交给 `WorldProject` 校验。
6. 当前 host 将十张地图投影为一张 `288x168` 的 `MapProject`。空坐标填充为不可走的空区。现有 `WorldApplication`、输入和相机因此可直接跨图移动。

领域 crate 仍不读取文件，也不保存相对路径。文件解析和地图拼接都在 `game-host` 的适配层。

## 当前限制

- host 当前一次加载十张地图，再投影为一张世界网格。这是让现有单 `TileMap` 玩法和相机先可用的兼容层。
- `preload_window` 已有纯领域实现，但 host 尚未按玩家坐标动态装卸九张地图。
- NPC、世界事件和存档仍按单世界网格处理。跨图独立状态与持久化还没有实现。

## 后续阶段

1. 在 `WorldApplication` 中维护当前坐标与 `MapStateCache`，发布 `MapWindowShifted`。
2. 在 host 中按窗口装卸九张地图；空槽走统一背景。
3. 增加 `Portal`、世界事件和版本化存档。

## 验收

- 纯 crate 不依赖文件系统、GPU、窗口或游戏 UI。
- `cargo test -p world-project` 通过。
- `python scripts/test_pure_coverage.py` 对该 crate 保持 100% 生产行覆盖率。
- `cargo test -p game-host` 验证区域清单、十图拼接和一处跨图移动。
