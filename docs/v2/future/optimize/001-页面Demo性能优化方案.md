# 页面 Demo 性能优化方案

> 分类：未来；起草：2026-07-26。
> 状态：性能方案评审。本文不表示优化已经实现。

实验数据统一使用[优化数据记录模板](000-优化数据记录模板.md)。

## 结论

当前卡顿的主要原因是页面模型重复构建，不是 UI 布局或 GPU 计划。

第一阶段应缓存嵌入数据，并在动画期间复用当前 `PageModel`。同时修正性能指标，把“实际渲染帧率”和“统计窗口帧率”分开记录。

当前日志的最终基线如下：

| 指标 | 实测值 |
| --- | ---: |
| 统计窗口 | 26.18s |
| 帧数 | 104 |
| 墙钟 FPS | 4.0 |
| 平均帧耗时 | 56.59ms |
| `model` | 47.92ms |
| `tree` | 0.32ms |
| `layout` | 0.06ms |
| `plan` | 0.06ms |
| `present` | 8.22ms |

`model` 占平均帧耗时约 84.7%。`tree`、`layout`、`plan` 合计约 0.44ms，不应作为第一优化目标。

## 性能事实

`game-page-demo` 每次重绘都会调用 `project_demo_page`。图鉴页面模型随后重复执行：

```text
PokedexData::embedded_gen3()
CurrentDataSet::embedded()
386 条图鉴 entry 重建
当前宝可梦技能列表重建
```

因此当前单帧至少包含两类工作：

1. 重新解析不变的嵌入数据。
2. 根据不变数据重新创建页面模型。

日志中的 4.0 FPS 不能直接等同于持续渲染能力。平均帧耗时 56.59ms 对应的活跃渲染上限约为 17.7 FPS，但 104 帧只占用了约 5.88 秒，剩余时间是事件循环没有请求重绘。

后续必须同时记录：

```text
active_fps       执行 redraw 的频率
wall_fps         统计窗口内的帧频
redraw_requests  请求重绘的次数
redraw_reason    input / animation / pointer / resize
animation_frames 动画期间的实际帧数
```

## 候选方案

### 方案 A：缓存嵌入数据

缓存 `PokedexData` 和 `CurrentDataSet` 的解析结果，保持现有 `PageModel` 构建逻辑。

按当前基线估算：

| 指标 | 当前 | 目标范围 |
| --- | ---: | ---: |
| `model` | 47.92ms | 2~8ms |
| 总帧耗时 | 56.59ms | 10.67~16.67ms |
| 活跃 FPS | 17.7 | 60~94 |

收益来自消除每帧的二进制解析和 JSON 解析。

优点是改动小、风险低、正式项目和 page demo 都能复用。缺点是 386 条 entry 和技能列表仍会每帧重建。

优先级：最高。

### 方案 B：缓存图鉴静态目录

将图鉴数据拆成两部分：

```text
PokedexCatalog
  386 条基础条目
  基础属性
  类型
  form 和 learnset 映射

PokedexPageModel
  selected
  selected_move
  stats_view
  detail_view
  当前已发现状态
```

静态目录只构建一次，页面模型只处理当前快照和焦点状态。

按当前基线估算：

| 指标 | 当前 | 目标范围 |
| --- | ---: | ---: |
| `model` | 47.92ms | 0.5~2ms |
| 总帧耗时 | 56.59ms | 9.07~10.67ms |
| 活跃 FPS | 17.7 | 94~110 |

优点是边界清楚，适合正式项目长期使用。缺点是需要定义快照变化后的缓存失效规则。

优先级：最高。建议在方案 A 验证后实施。

### 方案 C：缓存完整 `PageModel`

在 page demo 中保存当前页面模型：

```text
状态或 route 改变 -> 重建 PageModel
纯动画重绘       -> 复用 PageModel
鼠标 hover       -> 复用 PageModel
视差位置变化     -> 复用 PageModel
```

按当前基线估算：

| 指标 | 当前 | 目标范围 |
| --- | ---: | ---: |
| `model` | 47.92ms | 0~0.2ms |
| 总帧耗时 | 56.59ms | 8.42~8.87ms |
| 活跃 FPS | 17.7 | 113~119 |

优点是直接解决视差动画期间的重复模型构建。缺点是必须定义模型失效条件，正式游戏不能直接照搬 demo 的永久缓存。

优先级：高。建议和方案 A 配合。

### 方案 D：缓存 UI Tree 或 UiFrame

在模型缓存后，继续缓存不变页面的 UI 树或解析后的 frame，只更新焦点、交互状态和 `visual_offset`。

当前 `tree`、`layout`、`plan` 合计只有 0.44ms，因此当前基线最多节省约 0.44ms：

```text
56.59ms -> 56.15ms
17.7 FPS -> 17.8 FPS
```

方案 A/C 完成后，理论上可再节省约 0.5ms，收益约 5%~6%。

优点是适合后续 retained UI。缺点是容易让 action hit、interaction target 和视觉偏移使用不同版本。

优先级：低。

### 方案 E：修正重绘调度

检查以下链路：

- `redraw()` 内部调用 `advance_ui()`。
- `about_to_wait()` 再次调用 `advance_ui()`。
- 动画未结束时是否稳定设置 `WaitUntil`。
- 重绘请求是否在 `ControlFlow::Wait` 设置之前被吞掉。

该方案不会直接降低 56.59ms 的单帧耗时，但能让动画期间的墙钟 FPS 接近活跃 FPS。完成方案 C 后，动画活跃 FPS 应达到约 113 FPS，实际墙钟 FPS 应接近显示器刷新率或设定的动画频率。

优先级：高。必须和指标修正一起做。

### 方案 F：后台构建页面模型

将模型构建放到后台线程，主线程继续使用旧模型渲染和处理输入。

如果保留当前 48ms 模型构建，预期收益是：

```text
主线程输入阻塞: 约 48ms -> 约 0~2ms
旧模型渲染:     约 17.7 FPS -> 约 110 FPS
新模型可见延迟: 增加约 48ms
```

该方案不消除模型构建成本，还会引入模型版本、快照同步和旧结果覆盖新结果的问题。

优先级：低。只有模型缓存后仍超过 8~10ms，才考虑该方案。

### 方案 G：优化 GPU present

`present` 当前平均 8.22ms。即使完全消除，也只能得到：

```text
56.59ms -> 48.37ms
17.7 FPS -> 20.7 FPS
```

即使减少 2ms，活跃 FPS 也只有约 18.3。当前不应优先优化 GPU 提交。

优先级：最低。

## 实施顺序

```text
P0  增加 active FPS、redraw reason 和 model 子阶段指标
P1  缓存 PokedexData 和 CurrentDataSet
P2  缓存图鉴静态目录，重建动态页面状态
P3  page demo 缓存完整 PageModel，动画期间复用
P4  修正 WaitUntil 和重复 advance_ui 调度
P5  仍超过 8~10ms 时再评估后台线程
P6  最后评估 UI Tree、UiFrame 和 GPU present 缓存
```

默认第一轮实施 `P0 + P1 + P3`。如果 `model` 仍高于 5ms，再实施 `P2`。

## 代码边界

- `game-data`：提供嵌入数据的共享缓存入口。
- `game-page-model`：拆分图鉴静态目录和动态页面模型，定义缓存失效条件。
- `game-page-demo`：保存当前模型，按状态变化失效；完善性能统计和重绘原因。
- `game-view`：继续只消费 `PageModel`，不保存产品数据缓存。

## 验收标准

第一阶段：

```text
model 平均 <= 5ms
活跃帧耗时 <= 16.67ms
图鉴视差动画活跃 FPS >= 60
输入到焦点变化延迟 <= 16.67ms
墙钟 FPS 和活跃 FPS 分开显示
```

第二阶段：

```text
model 平均 <= 2ms
活跃帧耗时 <= 12ms
图鉴视差动画活跃 FPS >= 80
```

验证命令：

```text
ops format --check
ops lint
ops test --suite core
ops docs check
git diff --check
```

性能验证必须对比优化前后的 `model_ms`、`active_fps`、`wall_fps`、`animation_frames`，不能只看单一 FPS 数值。
