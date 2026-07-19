# JSON 与二进制存储

> 分类：现状；最后核对：2026-07-20。
> 依据：`map-project` JSON API、`map-project-storage` 与 `map-editor`/`map-editor-cli` 的读写分支。

## JSON 是可编辑表示

`MapProject` 提供 JSON 解析、漂亮格式化输出和项目校验。JSON 用于地图作者可阅读、可审阅的项目文件；`map-editor` 当前保存时调用 `to_json_pretty`，`map-editor-cli unpack` 也生成 JSON。

读取 JSON 时，调用方仍需提供已知原子图块 ID；项目模型会验证引用、地图结构和其他领域不变量。JSON 不是绕过地图校验的快速路径。

## `.g3mp` 是受限二进制容器

`map-project-storage` 的扩展名为 `g3mp`，magic 为 `G3MP`，当前容器版本为 1。它将 manifest 和编码后的地图载荷放入有固定头部的容器，并使用 Zstd 压缩和 BLAKE3 payload checksum。

| 限制 | 当前值 |
| --- | ---: |
| 头部长度 | 64 字节。 |
| 最大 manifest | 64 KiB。 |
| 最大压缩载荷 | 64 MiB。 |
| 最大原始载荷 | 256 MiB。 |
| 最大地图边长 | 4096。 |
| 最大 cell 数 | 4,194,304。 |

`MapProjectWriter::write` 在编码前校验项目和尺寸，压缩后写入长度与 checksum。`MapProjectReader::read` 依次检查头部、magic、版本、长度、manifest、解压、checksum 和解码结果。所有失败以 `MapStorageError` 表示。

## 调用方的格式选择

`map-editor` 加载时可依据扩展名读取 JSON 或 `.g3mp`，保存时写 JSON。`map-editor-cli` 加载和交互式保存时按扩展名选择格式；`pack` 明确要求 JSON 到 `.g3mp`，`unpack` 明确要求 `.g3mp` 到 JSON。文件读取、目录默认值和 `fs::write` 留在 runtime，容器 crate 不决定路径。

这两种表示共享 `MapProject` 的校验规则。格式不同不会改变地图的碰撞、事件、角色、瓦片引用或语义 lint 要求。
