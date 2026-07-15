---
name: knowledge-pattern-maintainer
description: 维护 Arbor 的经验沉淀、pattern 文档和可执行 skill。用于判断一次项目经验应该写入 workspace/learn/patterns、迭代日志、决策文档，还是抽象成 packages/arbor-skills 下的 agent skill；也用于整理来源证据、反模式和可复用规则。
---

# 知识模式维护器

用这个技能把项目经验整理成可复用知识。先判断沉淀位置，再写内容。

## 引用路由

- 判断写 pattern、skill、决策还是迭代日志：读 [capture-decision.md](references/capture-decision.md)。
- 写 `workspace/learn/patterns` 文档：读 [pattern-document.md](references/pattern-document.md)。
- 把经验升级成 skill：读 [skill-extraction.md](references/skill-extraction.md)。

## 默认流程

1. 先找来源证据，不凭印象写规则。
2. 判断经验是否会影响未来 agent 行动。
3. 只把稳定、可复用、有边界的经验写成规则。
4. 一次性历史写迭代日志，长期选择写决策文档。
5. 跨项目技术模式写 `workspace/learn/patterns`。
6. 需要在编码前触发的行动规则写进 `packages/arbor-skills`。

## 硬规则

- 不把整个知识库复制成 skill。
- 不把一次性 bug 或临时补丁写成长期规则。
- 证据和推断要分开。
- `SKILL.md` 保持短，详细规则放 `references/`。
- 新 skill 必须写 `skill.package.json` 并加入 `arbor.skills.json`。
