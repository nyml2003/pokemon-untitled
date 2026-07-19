# Adapter、Runtime 与 Quality

> 分类：现状；最后核对：2026-07-20。
> 依据：adapter/runtime/quality package 的 `Cargo.toml` 与公开根 API。

## Adapter 实现外部能力

adapter 的八个 crate 把基础模型和表现计划连接到具体 I/O、第三方库或受控能力。

| 组 | crate | 当前实现 |
| --- | --- | --- |
| 资产与地图 | `game-fs-assets`、`map-project-storage` | 本地目录读取；版本化 `.g3mp` 容器编解码。 |
| 导入与命令 | `game-data-import-core`、`game-data-import`、`battle-ramus-adapter` | CSV 导入规则与文件生成；授权的对战命令。 |
| 终端与图形 | `punctum-crossterm`、`punctum-wgpu`、`game-native-target` | 终端 I/O；Winit 输入/WGPU runtime；native 帧和文本提交。 |

adapter 将库错误转换为项目错误。`map-project-storage` 在读写前后执行容器和地图校验；`battle-ramus-adapter` 在执行前限制来源长度、调用数、能力和命令目录；`game-native-target` 将 GPU 与 glyphon 文本编码收束到一个提交边界。

## Runtime 装配可执行程序

`game-host`、`map-editor`、`map-editor-cli` 和 `tile-editor` 是四个 runtime package。它们选择路径、创建窗口或解析 CLI、创建 adapter、连接 application/presentation，并处理日志和退出。

runtime 可以依赖多层 crate，因为它是组合根。它不能成为第二个领域层：新增可复用规则或状态转换时，应当先判定其是否属于 domain 或 application；新增投影时，应当放入 presentation；新增外部实现时，应当放入 adapter。

## Quality 只验证

`game-e2e` 用 application 级依赖构造端到端游戏流程，不需要窗口或 GPU。`wslg-wgpu-clear-smoke` 只依赖 Winit、WGPU 和 pollster，用于检查 WSLg 图形环境。二者不被产品 runtime 依赖，也不定义产品 API。

修改共享状态、跨层数据合同或渲染提交边界时，质量工具是验证入口之一；它们不能被反向用作 runtime 的功能模块。
