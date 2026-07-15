# 技能抽取

## 意图

只把会影响 agent 行动的经验升级成 skill，保持 skill 集合小而准。

## 适用场景

- 某类任务会反复出现。
- 编码前需要加载特定规则。
- 有稳定流程、边界、反模式和验证命令。
- 用户明确要求把经验变成 skill。

## 必须遵守的规则

- `SKILL.md` 只写触发条件、路由、默认流程和硬规则。
- 详细规则放一层 `references/` 文件。
- front matter 只保留 `name` 和 `description`。
- `agents/openai.yaml` 用中文展示名和中文说明，`default_prompt` 必须包含 `$skill-name`。
- `skill.package.json` 必须显式列出全部文件。
- 加入 `packages/arbor-skills/arbor.skills.json` 后要刷新 lock。

## 推荐模式

- 一个 skill 覆盖一类任务，不覆盖一个文档标题。
- 一个 reference 覆盖一组相关规则。
- 从 pattern 文档提取规则时，保留证据路径，但不要全文复制。
- 优先提取硬约束、推荐流程、反模式和验证清单。

## 反模式

- `SKILL.md` 超长，像 README。
- 触发描述太窄，只匹配文件名。
- 触发描述太宽，所有任务都会加载。
- 把安装说明、历史复盘、设计争论全塞进 skill。

## 证据

- `skill-creator` 规则要求 `SKILL.md` 精简，详细材料放 `references/`。
- `packages/arbor-skills/skills/skill-manager-maintainer` 已采用短入口加 references 的结构。
