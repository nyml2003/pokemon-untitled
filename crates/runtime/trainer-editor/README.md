# trainer-editor

## 职责

`trainer-editor` 是训练师内容文件的命令行编辑器。它只读写指定 JSON 文件；训练师数据校验由 `game-foundation` 完成。

## 命令

`inspect <file>` 输出规范 JSON。`validate <file>` 检查格式和字段。

`set-name <file> <trainer-id> <name>` 修改姓名。
`set-script <file> <trainer-id> <script>` 修改交互脚本。
`add-pokemon <file> <trainer-id> <species> <level>` 增加队伍成员。
`replace-pokemon <file> <trainer-id> <slot> <species> <level>` 修改零起始槽位。
`remove-pokemon <file> <trainer-id> <slot>` 删除零起始槽位；不能删空队伍。

默认训练师文件为 `assets/source/trainer/trainers-v1.json`。`game-host` 和 Thin Slice 都会读取这个文件。
