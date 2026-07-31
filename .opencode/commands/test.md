---
description: 运行单元测试
---
在仓库根目录运行 `ops test`。可选参数指定 suite：`/test core` 对应 `ops test --suite core`，`/test world` 对应 `ops test --suite world`。若有失败，读取相关源码与测试定位问题并修复，重新运行对应 suite 验证。报告结果。
