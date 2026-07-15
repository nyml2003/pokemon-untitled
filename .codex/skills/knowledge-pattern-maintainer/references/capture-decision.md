# 沉淀位置判断

## 意图

把不同类型的经验放到合适位置，避免 `workspace/learn` 和 `packages/arbor-skills` 职责混乱。

## 适用场景

- 完成一次功能、重构或架构调整后整理经验。
- 用户要求“把经验沉淀下来”。
- 发现某个规则可能影响后续 agent 编码。

## 必须遵守的规则

- 一次性过程记录写 `workspace/learn/iteration-log`。
- 稳定技术模式写 `workspace/learn/patterns`。
- 长期项目选择写 `DECISIONS.md`。
- 仓库协作约定写 `CONVENTIONS.md`。
- 会直接影响后续 agent 行动的规则写成 skill 或 skill reference。
- 项目状态变化写 `README.md` 或 `PLAN.md`，不要塞进 skill。

## 推荐判断

- 如果读者是人，目的是理解历史：写文档。
- 如果读者是 agent，目的是编码前加载约束：写 skill。
- 如果规则只在一个 skill manager 任务中有用：写进对应 skill。
- 如果规则跨多个项目可复用：先写 pattern，再决定是否做 skill。

## 反模式

- 每个 pattern 都做成一个 skill。
- 把一段复盘完整复制进 `SKILL.md`。
- 没有来源证据就写“最佳实践”。
- 同一条规则在多个 skill 里重复维护。

## 证据

- `workspace/learn/patterns/README.md` 已按来源项目和主题索引模式。
- `arbor-repo-maintainer` 已承担仓库治理和经验捕获入口。
- `packages/arbor-skills` 已用于维护会影响 agent 行为的技能。
