# 领域建模

## 意图

把状态、身份、规则和生命周期集中到领域对象里，避免每个入口重复判断。

## 适用场景

- 任务、项目、文件、截图会话、安装计划等有状态对象。
- 需要明确状态转换和冲突处理。
- CLI、GUI、agent 会共享同一套业务规则。

## 必须遵守的规则

- 聚合根暴露状态转换方法，外部不能直接改内部状态。
- 构造入口负责验证必填字段和不变量。
- 状态转换失败返回结构化错误。
- 不同 ID 使用不同类型，不能靠裸 `string` 混传。
- 时间戳、状态、关联资源更新要在一个领域操作里完成。

## 推荐模式

- Rust 用 newtype ID；TypeScript 用 branded string。
- 状态机先写在聚合根方法里，不急着抽配置表。
- 领域错误用 discriminated union 或 enum 分类。
- Presenter 或 formatter 只负责展示，不参与业务判断。

## 反模式

- `task.status = "closed"` 这类外部直接赋值。
- `function closeTask(id: string)` 接收任何字符串。
- 抛 `Error("failed")` 后让调用方猜原因。
- 状态机散落在 CLI、UI 和测试 helper 里。

## 证据

- `workspace/learn/patterns/task-domain-model.md` 记录 workshop/workc 的 Aggregate Root、Repository、状态机和 Presenter 分离。
- `packages/skill-manager-core/src/domain/types.ts` 使用 branded string 区分 `SkillId`、`SkillName`、`SkillVersion`。
