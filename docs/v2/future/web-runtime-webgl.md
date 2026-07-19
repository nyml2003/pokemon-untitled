# Web runtime 与 WebGL 降级方案

> 分类：未来；最后整理：2026-07-20
> 状态：未实现；不代表已承诺排期。

## 结论

项目可以迁移到浏览器，并提供 WebGL2 降级渲染。

不应把现有渲染代码改写为直接调用 WebGL。应继续使用 `wgpu`：浏览器支持 WebGPU 时走 WebGPU；需要兼容较旧浏览器时，为 Wasm 构建启用 `wgpu` 的 `webgl` 后端，走 WebGL2。这样原生和 Web 共享 WGSL、纹理上传和大部分渲染管线定义。

当前 2D 渲染路径只使用实例缓冲、uniform、二维纹理、采样器和 alpha 混合。它是 WebGL2 可实现的基础能力集合。WebGL2 降级仍需单独验证，不能只通过桌面测试推断成功。

## 当前基础与边界

可以复用的部分：

- `domain`、`application`、`presentation` 和 `foundation` 中不依赖平台的状态、规则、场景投影、UI tree 与帧计划。
- `punctum-gpu` 的绘制数据模型。
- `punctum-wgpu` 中的 WGSL、sprite atlas 上传和实例绘制逻辑，经过目标平台条件编译后可作为 Web 渲染适配器的基础。

必须重新实现或拆分的部分：

| 现有实现 | Web 对应实现 | 原因 |
| --- | --- | --- |
| `game-host` 的 `winit` 事件循环 | `game-web-host` 的浏览器帧调度和 canvas 生命周期 | 浏览器由页面控制事件循环、焦点、可见性与尺寸。 |
| `game-native-target` 的 `pollster` 初始化 | 异步 Wasm 初始化 | 浏览器 GPU/资源读取都以异步 API 为主。 |
| `game-native-target` 的 `glyphon` 文本提交 | 独立 Web 文本渲染器 | 字体来源、布局和渲染资源不能假定原生环境。首版应采用预烘焙字形 atlas，或确认字体包在 Wasm 上可用后再复用。 |
| `game-fs-assets` 与 `std::fs::read` | HTTP `fetch` 或构建时打包的资源提供者 | 浏览器没有项目目录和任意文件路径访问权。 |
| 本地文件存储 | 初期只读资源；后续 IndexedDB 或用户显式导入/导出 | 浏览器持久化需要独立的数据所有权与失败模型。 |

浏览器 API、`web-sys`、Wasm 条件编译和网页资源协议只能出现在新的 adapter/runtime crate 中，不能进入 `foundation`、`domain`、`application` 或 `presentation`。

## 推荐 crate 结构

```text
runtime/game-web-host
  -> adapter/game-web-target
  -> adapter/game-web-assets
  -> presentation/application/domain/foundation
```

`game-web-host` 负责装配页面、canvas、请求动画帧、输入事件和启动错误显示。

`game-web-target` 把既有帧计划提交到 `wgpu`。它需要明确两套能力配置：`webgpu` 与 `webgl2`。适配器必须在初始化后记录实际后端，供诊断页面和测试读取。

`game-web-assets` 只提供资源字节与资源清单。它不得暴露文件系统路径，也不得让上层等待具体 HTTP 实现。

桌面 `game-host`、`game-native-target` 和 `game-fs-assets` 保持原状。Web 不是对它们增加大量 `cfg` 分支，而是独立的装配边界。

## 后端策略

| 模式 | 用途 | 要求 |
| --- | --- | --- |
| WebGPU | 默认路径 | 作为功能和性能基线。 |
| WebGL2 | 兼容降级 | 只承诺项目实际使用的 2D 功能；每次新增 GPU 特性都要重新确认兼容性。 |
| 无可用 GPU 后端 | 明确失败 | 显示可理解的启动错误，不伪造可运行状态。 |

不能在 WebGL2 路径中引入以下能力，除非同时提供替代实现并完成兼容测试：计算着色器、依赖高级纹理格式的资源、超过 WebGL2 最低限制的 bind group 或 uniform 布局、只在 WebGPU 可用的查询与同步能力。

WGSL 是共享着色器源。是否可在 WebGL2 后端运行，应以实际 Wasm 构建和目标浏览器测试为准，不以 API 名称判断。

## 分阶段实施

1. 建立 Wasm 空壳：新增 `game-web-host`，创建 canvas，初始化 `wgpu`，并记录 WebGPU/WebGL2 实际后端。此阶段不接入游戏逻辑。
2. 提取提交边界：将 `punctum-wgpu` 中与 `winit` 无关的渲染提交逻辑移到可被 Web target 使用的位置；原生输入转换仍保留在原生 adapter。
3. 接入静态资源：实现资源清单和异步加载，先加载一张 PNG atlas 与只读游戏数据。资源 URL、版本和加载失败必须可观察。
4. 接入最小游戏帧：复用 `game-session`、场景投影和帧计划，在 canvas 上显示地图、sprite 和基础 UI。
5. 接入输入和页面生命周期：处理键盘、指针、焦点、尺寸变化、暂停和页面隐藏；浏览器快捷键不得被无条件拦截。
6. 实现 WebGL2 降级并验收：强制选择 WebGL2 路径，逐项验证 sprite、透明混合、文本、缩放和输入。WebGPU 成功不能代替此验收。
7. 另立持久化提案：在游戏可运行后，再定义存档、地图编辑和浏览器存储的版本与迁移策略。

## 验收条件

首个可合入版本必须满足：

- `wasm32-unknown-unknown` 构建成功，并能由静态 HTTP 服务提供。
- 在支持 WebGPU 的目标浏览器中，canvas 能启动并显示最小场景。
- 在强制 WebGL2 的目标浏览器中，同一场景、同一 atlas 和同一输入合同可用。
- 资源缺失、网络失败、GPU 初始化失败与不支持的后端均返回可读错误。
- 核心层没有浏览器 API、文件系统路径或平台条件编译泄漏。
- 原生 `game-host` 的现有运行与测试不因 Web runtime 改动而改变。

## 需要先做的技术验证

在开始正式迁移前，应先完成一个独立的 2D Wasm 冒烟程序。它使用项目当前的 sprite shader、实例布局、PNG atlas 和 alpha 混合，分别跑在 WebGPU 与 WebGL2。通过后才创建 Web runtime crate。

这个验证应优先回答两个风险：当前 WGSL 和 pipeline 是否能在 `wgpu` 的 WebGL2 后端工作；文本渲染应使用何种可移植实现。两项都通过，迁移风险才主要落在资源加载和浏览器生命周期，而不是图形 API 本身。

## 参考

- [winit 的 Web 平台文档](https://docs.rs/winit/0.30.13/winit/platform/web/index.html)：`winit` 可编译到 `wasm32-unknown-unknown`，窗口由 HTML canvas 承载。
- [wgpu 30 的 Features 文档](https://wgpu.rs/doc/wgpu/struct.Features.html)：Web 与原生后端的能力并不完全相同，功能使用必须受目标能力约束。
