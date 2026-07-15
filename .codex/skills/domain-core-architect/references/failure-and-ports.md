# 失败模型与端口

## 意图

让正常失败可见、可测试、可恢复，让外部能力通过端口进入核心。

## 适用场景

- 文件不存在、权限不足、版本不匹配、输入无效、状态冲突。
- 需要替换文件系统、网络、剪贴板、时钟、进程执行等外部能力。
- 需要让 CLI 和 UI 展示同一类错误。

## 必须遵守的规则

- 正常失败返回 `Result`、diagnostic、error union 或等价结构。
- 只有不变量破坏和编程错误才抛异常或 panic。
- 端口接口由核心需要定义，不由具体实现倒推。
- Adapter 把平台错误转换成领域或应用错误。
- 测试要覆盖失败路径，不只测成功路径。

## 推荐模式

- `NotFound`、`AlreadyExists`、`InvalidInput`、`Conflict`、`PersistenceFailed` 分开。
- Driver 声明 capability，核心在调用前检查。
- Repository 返回 `Option`/`null` 表示“没找到”，不要把没找到当崩溃。
- CLI 用 exit code 和格式化输出表达错误；core 不知道终端。

## 反模式

- 用异常表示文件不存在。
- Adapter 接口照搬某个平台 API。
- 错误里只有字符串，没有 code、field、entity 或操作上下文。
- 测试只断言“抛了”，不检查错误类型。

## 证据

- `workspace/learn/patterns/vfs-pure-logic.md` 记录 Result-based error 和 Driver capability。
- `workspace/learn/patterns/task-domain-model.md` 记录 `DomainError` 分类。
- `packages/skill-manager-core/src/domain/diagnostics.ts` 和 validators 使用结构化 diagnostic。
