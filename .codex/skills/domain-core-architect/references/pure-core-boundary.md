# 纯核心边界

## 意图

让核心逻辑可测试、可移植、可复用。core 只表达业务和规则，不绑定运行环境。

## 适用场景

- 新建 `*-core` 包。
- 把文件系统、任务、截图、管理工具这类逻辑抽成可测试内核。
- 从 CLI/UI 里拆业务规则。

## 必须遵守的规则

- core 层不 import Node、Electron、Tauri、Win32、浏览器全局对象。
- core 层只依赖标准语言能力和明确传入的端口。
- 副作用通过 Driver、Repository、Service port 或 Adapter 注入。
- application 层编排 use case，不能把平台细节传回 domain。
- 测试优先使用内存实现，不依赖真实文件系统。

## 推荐模式

- 目录按 `domain/`、`application/`、`adapters/` 分层。
- public API 从 `index.ts` 或 crate/lib 入口统一导出。
- core 返回结构化结果，外壳负责展示、日志和进程退出。
- 路径、安全策略、权限检查集中在核心入口或 VFS 引擎，不散落到调用点。

## 反模式

- core 直接读写文件。
- CLI handler 里写状态转换。
- UI 组件直接修改领域对象。
- 为了赶进度把平台错误码传进 domain。

## 证据

- `workspace/learn/patterns/vfs-pure-logic.md` 记录 ObolosFS 的纯 VFS core 和 Driver 边界。
- `packages/skill-manager-core/src/domain`、`src/application`、`src/adapters` 已按领域、用例、Node 适配分层。
- `CONVENTIONS.md` 规定 core 不能依赖 CLI/UI。
