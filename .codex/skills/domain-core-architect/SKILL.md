---
name: domain-core-architect
description: 设计或维护纯领域核心、core 包、领域模型、状态机、Repository/Driver 边界、Result 错误模型和类型安全 ID。用于新增或重构核心逻辑、拆分 CLI/UI 与 domain、设计文件系统/任务/工作流这类可测试业务内核时。
---

# 领域核心架构师

用这个技能把业务规则放进纯核心层，把 IO、CLI、UI、平台 API 留在边界外。

## 引用路由

- 设计 core 包边界、隔离 IO 和平台 API：读 [pure-core-boundary.md](references/pure-core-boundary.md)。
- 设计任务、状态机、聚合根、类型安全 ID：读 [domain-modeling.md](references/domain-modeling.md)。
- 设计错误、端口、Repository 或 Driver：读 [failure-and-ports.md](references/failure-and-ports.md)。

## 默认流程

1. 先找业务不变量，不先写 CLI 或 UI。
2. 把正常失败建模成可返回的数据。
3. 给 ID、路径、状态这类易混概念建类型边界。
4. 把副作用收口到 Repository、Driver、Adapter 或 Service。
5. 用内存实现或 fixture 测试核心行为。
6. 最后再接 CLI、UI 或真实文件系统。

## 硬规则

- core 不直接 import Node、Electron、Tauri、Win32 或浏览器 API。
- CLI 和 UI 只能调用 application/use case，不写业务状态转换。
- 正常失败不要靠异常控制流。
- 不要用裸字符串传递不同种类的 ID 和路径。
- 先验证最小领域模型，再扩展外壳。
