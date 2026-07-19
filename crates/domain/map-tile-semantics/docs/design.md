# 设计说明

## 模块边界

`model` 负责序列化标识、规则、图案和目录数据。
`catalog` 负责目录校验、matcher 求值和地图 lint。
`diagnostic` 负责 lint 结果和无效目录错误。
`lib.rs` 导出稳定的领域 API，不暴露模块路径。

## 不变量

有效目录使用 `FORMAT_VERSION`，并且每个已知原子图块恰好定义一次。
图案 ID 和部件坐标在各自作用域内唯一。
`AnyOf` matcher 至少包含一个嵌套 matcher。
lint 将地图数据视为输入并返回诊断，不会修复数据。

## 演进约束

该 crate 不读取目录文件，也不知道资源路径。
adapter 提供 JSON 文本，调用方决定如何展示诊断。
